use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Read, Write};
use std::num::NonZeroU32;
use std::path::Path;

#[cfg(feature = "geo_ref")]
use crate::geo_referencing::CrsType;
use crate::symbols::{PublicOrPrivateSymbol, Symbol, SymbolId};

use geo_types::Coord;

use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, Event},
};

use crate::{
    colors::ColorSet,
    format_info::{OmapVersion, XmlDeclaration},
    geo_referencing::{GeoRef, MapTransform},
    notes,
    objects::MapObject,
    parts::MapPart,
    parts::MapParts,
    symbols::SymbolSet,
    templates::Templates,
    view::View,
    {Error, Result, ValidationError},
};

const DEFAULT_ISOM_15000: &[u8] = include_bytes!("default_maps/isom_15000.omap");
const DEFAULT_ISOM_10000: &[u8] = include_bytes!("default_maps/isom_10000.omap");
const DEFAULT_ISSPROM_4000: &[u8] = include_bytes!("default_maps/issprom_4000.omap");

/// The scale denominators the bundled default maps are drawn at.
/// Unwrap on consts is compile time
#[cfg(feature = "geo_ref")]
const SCALE_15_000: NonZeroU32 = NonZeroU32::new(15_000).unwrap();
#[cfg(feature = "geo_ref")]
const SCALE_10_000: NonZeroU32 = NonZeroU32::new(10_000).unwrap();
#[cfg(feature = "geo_ref")]
const SCALE_4_000: NonZeroU32 = NonZeroU32::new(4_000).unwrap();

/// All objects are in map coordinates i.e given in mm of paper
/// relative the ref point with positive y towards the magnetic north
///
/// The Undo/Redo history and printer information is ignored
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Omap {
    /// Free-text notes embedded in the file.
    pub notes: String,
    /// Georeferencing information (scale, CRS, reference points).
    pub geo_referencing: GeoRef,
    /// The ordered set of colors used by symbols.
    pub colors: ColorSet,
    /// The set of map symbols.
    pub symbols: SymbolSet,
    /// The map parts (layers) containing objects.
    pub parts: MapParts,
    /// Background templates attached to the map.
    pub templates: Templates,
    /// View settings (zoom, grid, visibility).
    pub view: View,
}

impl Omap {
    /// Create a new georeferenced `1:15_000` map with a complete ISOM symbolset and color order
    ///
    /// # Errors
    ///
    /// Returns an error if georeferencing cannot be initialized or the
    /// embedded default map cannot be parsed.
    #[cfg(feature = "geo_ref")]
    pub fn default_15_000_geo_referenced(
        projected_ref_point: Coord,
        crs: CrsType,
        meters_above_sea: f64,
    ) -> Result<Self> {
        let geo_ref = GeoRef::initialize(projected_ref_point, crs, meters_above_sea, SCALE_15_000)?;
        let mut omap = Self::from_bytes(DEFAULT_ISOM_15000)?;
        omap.geo_referencing = geo_ref;
        Ok(omap)
    }

    /// Create a new georeferenced `1:10_000` map with a complete ISOM symbolset and color order
    ///
    /// # Errors
    ///
    /// Returns an error if georeferencing cannot be initialized or the
    /// embedded default map cannot be parsed.
    #[cfg(feature = "geo_ref")]
    pub fn default_10_000_geo_referenced(
        projected_ref_point: Coord,
        crs: CrsType,
        meters_above_sea: f64,
    ) -> Result<Self> {
        let geo_ref = GeoRef::initialize(projected_ref_point, crs, meters_above_sea, SCALE_10_000)?;
        let mut omap = Self::from_bytes(DEFAULT_ISOM_10000)?;
        omap.geo_referencing = geo_ref;
        Ok(omap)
    }

    /// Create a new georeferenced `1:4_000` map with a complete `ISSprOM` symbolset and color order
    ///
    /// # Errors
    ///
    /// Returns an error if georeferencing cannot be initialized or the
    /// embedded default map cannot be parsed.
    #[cfg(feature = "geo_ref")]
    pub fn default_4_000_geo_referenced(
        projected_ref_point: Coord,
        crs: CrsType,
        meters_above_sea: f64,
    ) -> Result<Self> {
        let geo_ref = GeoRef::initialize(projected_ref_point, crs, meters_above_sea, SCALE_4_000)?;
        let mut omap = Self::from_bytes(DEFAULT_ISSPROM_4000)?;
        omap.geo_referencing = geo_ref;
        Ok(omap)
    }

    /// Create a new `1:15_000` map with a complete ISOM symbolset and color order
    ///
    /// # Errors
    ///
    /// Returns an error if the embedded default map cannot be parsed.
    pub fn default_15_000() -> Result<Self> {
        Self::from_bytes(DEFAULT_ISOM_15000)
    }

    /// Create a new `1:10_000` map with a complete ISOM symbolset and color order
    ///
    /// # Errors
    ///
    /// Returns an error if the embedded default map cannot be parsed.
    pub fn default_10_000() -> Result<Self> {
        Self::from_bytes(DEFAULT_ISOM_10000)
    }

    /// Create a new `1:4_000` map with a complete `ISSprOM` symbolset and color order
    ///
    /// # Errors
    ///
    /// Returns an error if the embedded default map cannot be parsed.
    pub fn default_4_000() -> Result<Self> {
        Self::from_bytes(DEFAULT_ISSPROM_4000)
    }

    /// Create a new empty map
    pub fn new(scale_denominator: NonZeroU32) -> Self {
        Self {
            notes: Default::default(),
            geo_referencing: GeoRef::new(scale_denominator),
            colors: ColorSet::new(),
            symbols: SymbolSet::new("Custom"),
            parts: MapParts::new_with_default_part(),
            templates: Default::default(),
            view: Default::default(),
        }
    }

    /// Construct an [`Omap`] from a byte sequence
    ///
    /// See [`Self::from_reader`] for more docs
    ///
    /// # Errors
    ///
    /// Returns an error if a required map section cannot be parsed.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        Self::from_reader(Cursor::new(bytes))
    }

    /// Construct an [`Omap`] from anything that implements [`Read`]
    ///
    /// Parsing is intentionally permissive for some sections.
    /// This function falls back to sensible defaults
    /// for `notes`, `templates`, or `view`
    /// if those sections cannot be parsed
    ///
    /// `barrier`s, `undo` and `redo` sections of the file are ignored
    ///
    /// The core sections `georeferencing`, `colors`, `symbols`, and `parts`
    /// must still parse successfully or else loading fails.
    ///
    /// # Errors
    ///
    /// Returns an error if a required map section cannot be parsed.
    pub fn from_reader<R: Read>(reader: R) -> Result<Self> {
        let mut reader = Reader::from_reader(BufReader::new(reader));
        reader.config_mut().expand_empty_elements = true;

        let mut georef = None;
        let mut colors = None;
        let mut symbols = None;
        let mut parts = None;

        let mut notes = String::new();
        let mut templates = Templates::default();
        let mut view = View::default();

        let mut xml_buf = Vec::new();
        loop {
            match reader.read_event_into(&mut xml_buf)? {
                Event::Decl(dec) => XmlDeclaration::parse(dec)?,
                Event::Start(bytes_start) => match bytes_start.local_name().as_ref() {
                    b"map" => OmapVersion::parse(&bytes_start)?,
                    b"notes" => notes = notes::parse(&mut reader).unwrap_or_default(),
                    b"georeferencing" => georef = Some(GeoRef::parse(&mut reader, &bytes_start)?),
                    b"colors" => colors = Some(ColorSet::parse(&mut reader, &bytes_start)?),
                    b"symbols" => {
                        if let Some(colors) = &colors {
                            symbols = Some(SymbolSet::parse(&mut reader, &bytes_start, colors)?);
                        } else {
                            return Err(Error::SectionOutOfOrder {
                                section: crate::OmapSection::Symbols,
                                required_before: crate::OmapSection::Colors,
                            });
                        }
                    }
                    b"parts" => {
                        if let Some(symbols) = &symbols {
                            parts = Some(MapParts::parse(&mut reader, symbols)?);
                        } else {
                            return Err(Error::SectionOutOfOrder {
                                section: crate::OmapSection::Parts,
                                required_before: crate::OmapSection::Symbols,
                            });
                        }
                    }
                    b"templates" => {
                        templates = Templates::parse(&mut reader, &bytes_start).unwrap_or_default();
                    }
                    b"view" => {
                        view = View::parse(&mut reader, &bytes_start, &mut templates)
                            .unwrap_or_default();
                    }
                    _ => (),
                },
                Event::End(bytes_end) => {
                    if bytes_end.local_name().as_ref() == b"map" {
                        break;
                    }
                }
                Event::Eof => break,
                _ => (),
            }
        }

        Ok(Self {
            notes,
            geo_referencing: georef.ok_or(Error::MissingRequiredSection(
                crate::OmapSection::Georeferencing,
            ))?,
            colors: colors.ok_or(Error::MissingRequiredSection(crate::OmapSection::Colors))?,
            symbols: symbols.ok_or(Error::MissingRequiredSection(crate::OmapSection::Symbols))?,
            parts: parts.ok_or(Error::MissingRequiredSection(crate::OmapSection::Parts))?,
            templates,
            view,
        })
    }

    /// Create an [`Omap`] from a path to an `.omap` file.
    ///
    /// See [`Self::from_reader`] for more docs
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or a required map section
    /// cannot be parsed.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)?;
        Self::from_reader(BufReader::new(file))
    }

    /// Write the map to anything that implements [`Write`]
    ///
    /// Symbols are written in the order the symbol set holds them. Call
    /// [`crate::symbols::SymbolSet::sort`] first to write them by
    /// [`crate::Code`].
    ///
    /// # Errors
    ///
    /// Returns an error if any of the map data cannot be serialized.
    pub fn to_writer<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut writer = Writer::new(writer);

        XmlDeclaration::write(&mut writer)?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        OmapVersion::write(&mut writer)?;
        writer.get_mut().write_all(b"\n".as_slice())?;

        notes::write(self.notes.as_str(), &mut writer)?;
        writer.get_mut().write_all(b"\n".as_slice())?;

        self.geo_referencing.write(&mut writer)?;
        writer.get_mut().write_all(b"\n".as_slice())?;

        // write colors
        self.colors.write(&mut writer)?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        // write symbols
        self.symbols.write(&mut writer, &self.colors)?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        // write objects
        self.parts.write(&mut writer, &self.symbols)?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        // write templates
        let vis = self.templates.write(&mut writer)?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        // write view
        self.view.write(&mut writer, vis)?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        // write eof
        writer.write_event(Event::End(BytesEnd::new("map")))?;
        writer.get_mut().flush()?;
        Ok(())
    }

    /// Write the map to an `.omap` file at the given path.
    ///
    /// See [`Self::to_writer`] for more docs
    ///
    /// The replacement is atomic on platforms where [`std::fs::rename`] atomically
    /// replaces an existing destination. The temporary file is created beside
    /// `path`, so it is always on the same filesystem as the destination.
    ///
    /// # Errors
    ///
    /// Returns an error if the temporary file cannot be created, map data
    /// cannot be serialized, or the temporary file cannot replace `path`.
    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let file_name = path.file_name().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "OMAP output path must name a file",
            )
        })?;

        let mut temporary_name = file_name.to_os_string();
        temporary_name.push(format!(".omap-rs-{}.tmp", std::process::id()));
        let temporary_path = parent.join(temporary_name);
        let temporary_file = File::create(&temporary_path)?;

        let write_result = {
            let mut writer = BufWriter::new(temporary_file);
            self.to_writer(&mut writer)
        };
        if let Err(error) = write_result {
            std::fs::remove_file(temporary_path)?;
            return Err(error);
        }

        if let Err(error) = std::fs::rename(&temporary_path, path) {
            std::fs::remove_file(temporary_path)?;
            return Err(error.into());
        }
        Ok(())
    }

    /// Iterate through all objects of every map part in a flat iterator.
    ///
    /// The whole-map counterpart of [`MapPart::iter_all_objects`]; parts are
    /// visited in order, objects within a part in no particular order.
    pub fn iter_all_objects(&self) -> impl Iterator<Item = &MapObject> {
        self.parts.iter().flat_map(MapPart::iter_all_objects)
    }

    /// Iterate mutably through all objects of every map part in a flat iterator.
    ///
    /// The whole-map counterpart of [`MapPart::iter_all_objects_mut`]; parts are
    /// visited in order, objects within a part in no particular order.
    pub fn iter_all_objects_mut(&mut self) -> impl Iterator<Item = &mut MapObject> {
        self.parts
            .iter_mut()
            .flat_map(MapPart::iter_all_objects_mut)
    }

    /// Transform every object and non-georeferenced template in the map.
    ///
    /// Use this after changing the georeferencing
    /// to keep objects and non-georeferenced templates at the same real-world
    /// positions. Obtain the transform with
    /// [`MapTransform::transform_between`].
    pub fn transform<F>(&mut self, transform: F)
    where
        F: Fn(Coord) -> Coord,
    {
        for object in self.iter_all_objects_mut() {
            object.transform(&transform);
        }
        self.templates.transform(transform);
    }

    /// Try to transform every object and non-georeferenced template in the map.
    ///
    /// Use this after changing the georeferencing to keep objects and
    /// non-georeferenced templates at the same real-world positions. The map is
    /// unchanged on failure.
    ///
    /// # Errors
    ///
    /// Returns any error produced while transforming an object or template.
    pub fn try_transform<E, F>(&mut self, transform: F) -> std::result::Result<(), E>
    where
        F: Fn(Coord) -> std::result::Result<Coord, E>,
    {
        let mut parts = self.parts.clone();
        for part in &mut parts {
            for object in part.iter_all_objects_mut() {
                object.try_transform(&transform)?;
            }
        }
        let mut templates = self.templates.clone();
        templates.try_transform(transform)?;

        self.parts = parts;
        self.templates = templates;
        Ok(())
    }

    /// Compute the transform between two [`MapTransform`]s and apply it
    /// to every object and non-georeferenced template. This is a convenience
    /// wrapper around [`MapTransform::transform_between`] + [`Omap::try_transform`].
    ///
    /// # Errors
    ///
    /// Returns an error if the transforms cannot be related or a coordinate
    /// cannot be transformed. The map is unchanged on failure.
    pub fn try_transform_between(&mut self, old: &MapTransform, new: &MapTransform) -> Result<()> {
        let transform = MapTransform::transform_between(old, new)?;
        self.try_transform(transform)
    }

    /// Drop every reference to a symbol or color that is no longer in its set.
    ///
    /// A dangling symbol reference becomes `None`, which writes as the format's
    /// `-1`; a dangling color reference becomes
    /// [`crate::colors::SymbolColor::NoColor`]; and a dangling combined-symbol
    /// component or mixed-color component is dropped.
    ///
    /// Handles you already hold stay valid across this call — only the map's own
    /// references to removed values change. [`Omap::validate`] reports the
    /// references this removes.
    pub fn prune_dangling_references(&mut self) {
        let live = crate::prune::Live {
            symbols: self.symbols.ids().map(|id| id.0).collect(),
            colors: self.colors.ids().map(|id| id.0).collect(),
        };

        use crate::prune::Prune as _;
        for color in self.colors.values_mut() {
            color.prune(&live);
        }
        for symbol in self.symbols.values_mut() {
            symbol.prune(&live);
        }
        for object in self.iter_all_objects_mut() {
            object.prune(&live);
        }
    }

    /// Validate references between objects, symbols, and colors.
    ///
    /// # Errors
    ///
    /// Returns the first invalid reference with its map location, or an error
    /// if a symbol cannot be borrowed during validation.
    pub fn validate(&self) -> std::result::Result<(), ValidationError> {
        for (symbol_index, symbol) in self.symbols.values().enumerate() {
            for color in symbol.colors(&self.symbols) {
                if !self.colors.contains(color) {
                    return Err(ValidationError::DanglingSymbolColor { symbol_index });
                }
            }

            let components: Vec<Option<SymbolId>> = match symbol {
                Symbol::CombinedArea(combined) => combined
                    .components()
                    .map(|part| match part {
                        PublicOrPrivateSymbol::Public(id) => Some(SymbolId::from(*id)),
                        PublicOrPrivateSymbol::Private(_) => None,
                    })
                    .collect(),
                Symbol::CombinedLine(combined) => combined
                    .components()
                    .map(|part| match part {
                        PublicOrPrivateSymbol::Public(id) => Some(SymbolId::from(*id)),
                        PublicOrPrivateSymbol::Private(_) => None,
                    })
                    .collect(),
                _ => continue,
            };
            for (component_index, component) in components.into_iter().enumerate() {
                if let Some(component) = component
                    && !self.symbols.contains(component)
                {
                    return Err(ValidationError::DanglingCombinedComponent {
                        symbol_index,
                        component_index,
                    });
                }
            }
        }

        for (object_index, object) in self.iter_all_objects().enumerate() {
            if let Some(symbol) = object.symbol()
                && !self.symbols.contains(symbol)
            {
                return Err(ValidationError::DanglingObjectSymbol { object_index });
            }
        }
        Ok(())
    }
}

#[expect(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use std::{fs, num::NonZeroU32};

    use geo_types::{Coord, Point};

    use super::Omap;
    use crate::{Error, Result, ValidationError, objects::PointObject};

    fn point_positions(map: &Omap) -> Vec<Coord> {
        map.iter_all_objects()
            .filter_map(|object| match object {
                crate::objects::MapObject::Point(point) => Some(point.geometry().0),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn try_transform_is_transactional() -> Result<()> {
        let mut map = Omap::new(NonZeroU32::new(10_000).unwrap());
        let part = map.parts.get_mut(0).ok_or(Error::ObjectError)?;
        part.add_object(PointObject::new(None, Point::new(1.0, 2.0)));
        part.add_object(PointObject::new(None, Point::new(10.0, 20.0)));
        let before = point_positions(&map);

        let result = map.try_transform(|coord| {
            if coord.x > 5.0 {
                Err(Error::ObjectError)
            } else {
                Ok(Coord {
                    x: coord.x + 100.0,
                    y: coord.y,
                })
            }
        });

        assert!(matches!(result, Err(Error::ObjectError)));
        assert_eq!(point_positions(&map), before);
        Ok(())
    }

    #[test]
    fn validate_reports_the_dangling_object_location() -> Result<()> {
        let mut map = Omap::new(NonZeroU32::new(10_000).ok_or(Error::ObjectError)?);
        let point = map
            .symbols
            .add_point_symbol(crate::symbols::PointSymbol::new(
                crate::Code::new(1, 0, 0),
                "dot",
            ));

        let part = map.parts.get_mut(0).ok_or(Error::ObjectError)?;
        part.add_object(PointObject::new(Some(point), Point::new(1.0, 2.0)));
        assert!(map.validate().is_ok(), "a live handle must validate");

        let _removed = map.symbols.remove(point.into());
        assert!(
            matches!(
                map.validate(),
                Err(ValidationError::DanglingObjectSymbol { object_index: 0 })
            ),
            "a handle to a removed symbol must not validate"
        );
        Ok(())
    }

    #[test]
    fn an_object_without_a_symbol_validates() -> Result<()> {
        let mut map = Omap::new(NonZeroU32::new(10_000).ok_or(Error::ObjectError)?);
        let part = map.parts.get_mut(0).ok_or(Error::ObjectError)?;
        part.add_object(PointObject::new(None, Point::new(1.0, 2.0)));

        assert!(map.validate().is_ok());
        Ok(())
    }

    #[test]
    fn to_file_preserves_existing_file_when_serialization_fails() -> Result<()> {
        let path = std::env::temp_dir().join(format!(
            "omap-rs-atomic-write-test-{}.omap",
            std::process::id(),
        ));
        fs::write(&path, b"previous map")?;

        let mut map = Omap::new(NonZeroU32::new(10_000).ok_or(Error::ObjectError)?);
        let part = map.parts.get_mut(0).ok_or(Error::ObjectError)?;
        part.add_object(PointObject::new(None, Point::new(3_000_000.0, 0.0)));

        let result = map.to_file(&path);
        assert!(matches!(result, Err(Error::MapCoordOutOfBounds)));
        assert_eq!(fs::read(&path)?, b"previous map");
        fs::remove_file(path)?;
        Ok(())
    }

    /// A dangling reference must prune to `None`, never to whatever value later
    /// takes the removed symbol's place.
    #[test]
    fn pruning_drops_references_to_removed_symbols() -> Result<()> {
        let mut map = Omap::from_bytes(super::DEFAULT_ISOM_15000)?;
        let point = map
            .symbols
            .iter_point_symbols()
            .next()
            .map(|(id, _)| id)
            .unwrap();

        let part = map.parts.get_mut(0).unwrap();
        part.add_object(PointObject::new(Some(point), Point::new(1.0, 2.0)));

        let _removed = map.symbols.remove(point.into());
        assert!(map.validate().is_err(), "the object now dangles");

        map.prune_dangling_references();

        let object = map.iter_all_objects().last().unwrap();
        assert_eq!(
            object.symbol(),
            None,
            "a dangling reference must prune to None"
        );
        assert!(map.validate().is_ok());
        Ok(())
    }
}
