use crate::Scale;
use std::fmt;

/// Orienteering map symbols higher order enum
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Symbol {
    /// Symbols for area objects
    Area(AreaSymbol),
    /// Symbols for line objects
    Line(LineSymbol),
    /// Symbols for point objects
    Point(PointSymbol),
    /// Symbols for text objects
    Text(TextSymbol),
}

/// Symbols for area objects
#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AreaSymbol {
    BrokenGround,
    VeryBrokenGround,
    GiganticBoulder,
    BoulderField,
    DenseBoulderField,
    StonyGroundSlow,
    StonyGroundWalk,
    StonyGroundFight,
    SandyGround,
    BareRock,
    UncrossableWaterWithBankLine,
    UncrossableWaterWithoutBankLine,
    UncrossableWaterDominantWithBankLine,
    UncrossableWaterDominantWithoutBankLine,
    ShallowWaterWithSolidBankLine,
    ShallowWaterWithDashedBankLine,
    ShallowWaterWithoutBankLine,
    SmallShallowWater,
    UncrossableMarshWithBankLine,
    UncrossableMarshWithoutBankLine,
    Marsh,
    IndistinctMarsh,
    OpenLand,
    OpenLandScatteredTrees,
    OpenLandScatteredBushes,
    RoughOpenLand,
    RoughOpenLandScatteredTrees,
    RoughOpenLandScatteredBushes,
    Forest,
    LightGreen,
    LightGreenOneDirectionWhite,
    UnderGrowth,
    MediumGreen,
    MediumGreenOneDirectionWhite,
    MediumGreenOneDirectionLightGreen,
    DenseUnderGrowth,
    DarkGreen,
    DarkGreenOneDirectionWhite,
    DarkGreenOneDirectionLightGreen,
    DarkGreenOneDirectionMediumGreen,
    CultivatedLand,
    Orchard,
    RoughOrchard,
    Vineyard,
    RoughVineyard,
    PavedAreaWithBoundary,
    PavedAreaWithoutBoundary,
    PrivateArea,
    Building,
    LargeBuildingWithOutline,
    LargeBuildingWithoutOutline,
    CanopyWithOutline,
    CanopyWithoutOutline,
    MagneticNorthBlack,
    MagneticNorthBlue,
    OutOfBounds,
}

/// Symbols for line objects
#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LineSymbol {
    Contour,
    BasemapContour,
    NegBasemapContour,
    IndexContour,
    Formline,
    EarthBank,
    EarthBankTopLine,
    EarthBankTagLine,
    EarthWall,
    RetainingEarthWall,
    RuinedEarthWall,
    ErosionGully,
    SmallErosionGully,
    ImpassableCliff,
    ImpassableCliffTopLine,
    ImpassableCliffTagLine,
    Cliff,
    CliffWithTags,
    Trench,
    BankLine,
    ShallowWaterOutline,
    ShallowWaterDashedOutline,
    CrossableWatercourse,
    SmallCrossableWatercourse,
    SeasonalWatercourse,
    NarrowMarsh,
    Hedge,
    DistinctCultivationBoundary,
    DistinctVegetationBoundary,
    PavedAreaBoundingLine,
    Road,
    RoadDualCarriageway,
    GravelRoad,
    VehicleTrack,
    Footpath,
    SmallFootpath,
    IndistinctFootpath,
    NarrowRide,
    NarrowRideEasyRunning,
    NarrowRideNormalRunning,
    NarrowRideSlowRunning,
    NarrowRideWalk,
    Railway,
    ImpassableRailway,
    SmallPowerline,
    Powerline,
    MajorPowerline,
    MajorPowerlineWithPylons,
    BridgeTunnel,
    Wall,
    RetainingWall,
    RuinedWall,
    ImpassableWall,
    Fence,
    RuinedFence,
    ImpassableFence,
    PrivateAreaBoundingLine,
    LargeBuildingOutline,
    CanopyOutline,
    Ruin,
    ProminentLinearFeature,
    ImpassableProminentLinearFeature,
    Stairway,
    SimpleOrienteeringCourse,
}

/// Symbols for point objects
#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PointSymbol {
    SlopelineContour,
    SlopelineFormline,
    MinimumEarthBank,
    DotKnoll,
    ElongatedDotKnoll,
    UDepression,
    Pit,
    BrokenGroundSingleDot,
    ProminentLandFeature,
    MinimumImpassableCliff,
    MinimumCliff,
    MinimumCliffWithTags,
    RockyPitCave,
    DangerousPit,
    SmallBoulder,
    MediumBoulder,
    LargeBoulder,
    BoulderCluster,
    LargeBoulderCluster,
    BoulderFieldSingleTriangle,
    BoulderFieldSingleTriangleLarge,
    StonyGroundSingleDot,
    Waterhole,
    MinimumMarsh,
    MinimumIndistinctMarsh,
    Well,
    Spring,
    ProminentWaterFeature,
    ProminentTree,
    ProminentBush,
    ProminentVegetationFeature,
    MinimumBridgeTunnel,
    Footbridge,
    FenceCrossingPoint,
    MinimumBuilding,
    MinimumRuin,
    HighTower,
    Tower,
    Cairn,
    FodderRack,
    ProminentManMadeFeatureO,
    ProminentManMadeFeatureX,
    RegistrationMark,
    SpotHeight,
    OpenOrienteeringMapperLogo,
}

/// Symbols for text objects
#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TextSymbol {
    ContourValue,
    SpotHeight,
    ControlNumber,
}

impl Symbol {
    /// Check if symbol is a line symbol
    pub fn is_line_symbol(&self) -> bool {
        matches!(self, Symbol::Line(_))
    }

    /// Check if symbol is a point symbol
    pub fn is_point_symbol(&self) -> bool {
        matches!(self, Symbol::Point(_))
    }

    /// Check if symbol is an area symbol
    pub fn is_area_symbol(&self) -> bool {
        matches!(self, Symbol::Area(_))
    }

    /// Check if symbol is a text symbol
    pub fn is_text_symbol(&self) -> bool {
        matches!(self, Symbol::Text(_))
    }

    /// get an iterator over all the symbols in draw order
    /// (from bottom to top, assuming all symbols are a single color)
    pub fn iter_in_draw_order() -> std::vec::IntoIter<Symbol> {
        vec![
            Symbol::Area(AreaSymbol::RoughOpenLand),
            Symbol::Area(AreaSymbol::SandyGround),
            Symbol::Area(AreaSymbol::BareRock),
            Symbol::Area(AreaSymbol::LightGreen),
            Symbol::Area(AreaSymbol::MediumGreen),
            Symbol::Area(AreaSymbol::DarkGreen),
            Symbol::Area(AreaSymbol::Marsh),
            Symbol::Area(AreaSymbol::PavedAreaWithBoundary),
            Symbol::Line(LineSymbol::BasemapContour),
            Symbol::Line(LineSymbol::Contour),
            Symbol::Line(LineSymbol::IndexContour),
            Symbol::Line(LineSymbol::Formline),
            Symbol::Line(LineSymbol::NegBasemapContour),
            Symbol::Point(PointSymbol::SlopelineContour),
            Symbol::Point(PointSymbol::SlopelineFormline),
            Symbol::Point(PointSymbol::DotKnoll),
            Symbol::Point(PointSymbol::ElongatedDotKnoll),
            Symbol::Point(PointSymbol::UDepression),
            Symbol::Area(AreaSymbol::UncrossableWaterDominantWithBankLine),
            Symbol::Area(AreaSymbol::GiganticBoulder),
            Symbol::Point(PointSymbol::SmallBoulder),
            Symbol::Point(PointSymbol::LargeBoulder),
            Symbol::Area(AreaSymbol::Building),
        ]
        .into_iter()
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Symbol::Area(area_symbol) => write!(f, "{:?}", area_symbol),
            Symbol::Line(line_symbol) => write!(f, "{:?}", line_symbol),
            Symbol::Point(point_symbol) => write!(f, "{:?}", point_symbol),
            Symbol::Text(text_symbol) => write!(f, "{:?}", text_symbol),
        }
    }
}

impl From<AreaSymbol> for Symbol {
    fn from(value: AreaSymbol) -> Self {
        Symbol::Area(value)
    }
}

impl From<LineSymbol> for Symbol {
    fn from(value: LineSymbol) -> Self {
        Symbol::Line(value)
    }
}

impl From<PointSymbol> for Symbol {
    fn from(value: PointSymbol) -> Self {
        Symbol::Point(value)
    }
}

impl From<TextSymbol> for Symbol {
    fn from(value: TextSymbol) -> Self {
        Symbol::Text(value)
    }
}

/// trait defining the three functions all symbol types must have
pub trait SymbolTrait {
    /// minimum size of an object with the symbol at the scale
    fn min_size(&self, _scale: Scale) -> f64 {
        0.
    }

    /// some symbols are rotateable or their pattern are rotateable
    fn is_rotateable(&self) -> bool {
        false
    }

    /// the id of the symbol in the symbol_x.txt files
    fn id(&self) -> u8;
}

impl SymbolTrait for Symbol {
    fn min_size(&self, scale: Scale) -> f64 {
        match self {
            Symbol::Area(a) => a.min_size(scale),
            Symbol::Line(l) => l.min_size(scale),
            Symbol::Point(p) => p.min_size(scale),
            Symbol::Text(t) => t.min_size(scale),
        }
    }

    fn id(&self) -> u8 {
        match self {
            Symbol::Area(a) => a.id(),
            Symbol::Line(l) => l.id(),
            Symbol::Point(p) => p.id(),
            Symbol::Text(t) => t.id(),
        }
    }

    fn is_rotateable(&self) -> bool {
        match self {
            Symbol::Area(a) => a.is_rotateable(),
            Symbol::Line(l) => l.is_rotateable(),
            Symbol::Point(p) => p.is_rotateable(),
            Symbol::Text(t) => t.is_rotateable(),
        }
    }
}

impl SymbolTrait for AreaSymbol {
    // in square meters
    fn min_size(&self, scale: Scale) -> f64 {
        match scale {
            Scale::S10_000 => match self {
                AreaSymbol::BrokenGround => todo!(),
                AreaSymbol::VeryBrokenGround => todo!(),
                AreaSymbol::GiganticBoulder => todo!(),
                AreaSymbol::BoulderField => todo!(),
                AreaSymbol::DenseBoulderField => todo!(),
                AreaSymbol::StonyGroundSlow => todo!(),
                AreaSymbol::StonyGroundWalk => todo!(),
                AreaSymbol::StonyGroundFight => todo!(),
                AreaSymbol::SandyGround => todo!(),
                AreaSymbol::BareRock => todo!(),
                AreaSymbol::UncrossableWaterWithBankLine => todo!(),
                AreaSymbol::UncrossableWaterWithoutBankLine => todo!(),
                AreaSymbol::UncrossableWaterDominantWithBankLine => todo!(),
                AreaSymbol::UncrossableWaterDominantWithoutBankLine => todo!(),
                AreaSymbol::ShallowWaterWithSolidBankLine => todo!(),
                AreaSymbol::ShallowWaterWithDashedBankLine => todo!(),
                AreaSymbol::ShallowWaterWithoutBankLine => todo!(),
                AreaSymbol::SmallShallowWater => todo!(),
                AreaSymbol::UncrossableMarshWithBankLine => todo!(),
                AreaSymbol::UncrossableMarshWithoutBankLine => todo!(),
                AreaSymbol::Marsh => todo!(),
                AreaSymbol::IndistinctMarsh => todo!(),
                AreaSymbol::OpenLand => todo!(),
                AreaSymbol::OpenLandScatteredTrees => todo!(),
                AreaSymbol::OpenLandScatteredBushes => todo!(),
                AreaSymbol::RoughOpenLand => todo!(),
                AreaSymbol::RoughOpenLandScatteredTrees => todo!(),
                AreaSymbol::RoughOpenLandScatteredBushes => todo!(),
                AreaSymbol::Forest => todo!(),
                AreaSymbol::LightGreen => todo!(),
                AreaSymbol::LightGreenOneDirectionWhite => todo!(),
                AreaSymbol::UnderGrowth => todo!(),
                AreaSymbol::MediumGreen => todo!(),
                AreaSymbol::MediumGreenOneDirectionWhite => todo!(),
                AreaSymbol::MediumGreenOneDirectionLightGreen => todo!(),
                AreaSymbol::DenseUnderGrowth => todo!(),
                AreaSymbol::DarkGreen => todo!(),
                AreaSymbol::DarkGreenOneDirectionWhite => todo!(),
                AreaSymbol::DarkGreenOneDirectionLightGreen => todo!(),
                AreaSymbol::DarkGreenOneDirectionMediumGreen => todo!(),
                AreaSymbol::CultivatedLand => todo!(),
                AreaSymbol::Orchard => todo!(),
                AreaSymbol::RoughOrchard => todo!(),
                AreaSymbol::Vineyard => todo!(),
                AreaSymbol::RoughVineyard => todo!(),
                AreaSymbol::PavedAreaWithBoundary => todo!(),
                AreaSymbol::PavedAreaWithoutBoundary => todo!(),
                AreaSymbol::PrivateArea => todo!(),
                AreaSymbol::Building => todo!(),
                AreaSymbol::LargeBuildingWithOutline => todo!(),
                AreaSymbol::LargeBuildingWithoutOutline => todo!(),
                AreaSymbol::CanopyWithOutline => todo!(),
                AreaSymbol::CanopyWithoutOutline => todo!(),
                AreaSymbol::MagneticNorthBlack => todo!(),
                AreaSymbol::MagneticNorthBlue => todo!(),
                AreaSymbol::OutOfBounds => todo!(),
            },
            Scale::S15_000 => match self {
                AreaSymbol::BrokenGround => todo!(),
                AreaSymbol::VeryBrokenGround => todo!(),
                AreaSymbol::GiganticBoulder => todo!(),
                AreaSymbol::BoulderField => todo!(),
                AreaSymbol::DenseBoulderField => todo!(),
                AreaSymbol::StonyGroundSlow => todo!(),
                AreaSymbol::StonyGroundWalk => todo!(),
                AreaSymbol::StonyGroundFight => todo!(),
                AreaSymbol::SandyGround => todo!(),
                AreaSymbol::BareRock => todo!(),
                AreaSymbol::UncrossableWaterWithBankLine => todo!(),
                AreaSymbol::UncrossableWaterWithoutBankLine => todo!(),
                AreaSymbol::UncrossableWaterDominantWithBankLine => todo!(),
                AreaSymbol::UncrossableWaterDominantWithoutBankLine => todo!(),
                AreaSymbol::ShallowWaterWithSolidBankLine => todo!(),
                AreaSymbol::ShallowWaterWithDashedBankLine => todo!(),
                AreaSymbol::ShallowWaterWithoutBankLine => todo!(),
                AreaSymbol::SmallShallowWater => todo!(),
                AreaSymbol::UncrossableMarshWithBankLine => todo!(),
                AreaSymbol::UncrossableMarshWithoutBankLine => todo!(),
                AreaSymbol::Marsh => todo!(),
                AreaSymbol::IndistinctMarsh => todo!(),
                AreaSymbol::OpenLand => todo!(),
                AreaSymbol::OpenLandScatteredTrees => todo!(),
                AreaSymbol::OpenLandScatteredBushes => todo!(),
                AreaSymbol::RoughOpenLand => todo!(),
                AreaSymbol::RoughOpenLandScatteredTrees => todo!(),
                AreaSymbol::RoughOpenLandScatteredBushes => todo!(),
                AreaSymbol::Forest => todo!(),
                AreaSymbol::LightGreen => todo!(),
                AreaSymbol::LightGreenOneDirectionWhite => todo!(),
                AreaSymbol::UnderGrowth => todo!(),
                AreaSymbol::MediumGreen => todo!(),
                AreaSymbol::MediumGreenOneDirectionWhite => todo!(),
                AreaSymbol::MediumGreenOneDirectionLightGreen => todo!(),
                AreaSymbol::DenseUnderGrowth => todo!(),
                AreaSymbol::DarkGreen => todo!(),
                AreaSymbol::DarkGreenOneDirectionWhite => todo!(),
                AreaSymbol::DarkGreenOneDirectionLightGreen => todo!(),
                AreaSymbol::DarkGreenOneDirectionMediumGreen => todo!(),
                AreaSymbol::CultivatedLand => todo!(),
                AreaSymbol::Orchard => todo!(),
                AreaSymbol::RoughOrchard => todo!(),
                AreaSymbol::Vineyard => todo!(),
                AreaSymbol::RoughVineyard => todo!(),
                AreaSymbol::PavedAreaWithBoundary => todo!(),
                AreaSymbol::PavedAreaWithoutBoundary => todo!(),
                AreaSymbol::PrivateArea => todo!(),
                AreaSymbol::Building => todo!(),
                AreaSymbol::LargeBuildingWithOutline => todo!(),
                AreaSymbol::LargeBuildingWithoutOutline => todo!(),
                AreaSymbol::CanopyWithOutline => todo!(),
                AreaSymbol::CanopyWithoutOutline => todo!(),
                AreaSymbol::MagneticNorthBlack => todo!(),
                AreaSymbol::MagneticNorthBlue => todo!(),
                AreaSymbol::OutOfBounds => todo!(),
            },
        }
    }

    fn id(&self) -> u8 {
        match self {
            AreaSymbol::BrokenGround => 21,
            AreaSymbol::VeryBrokenGround => 23,
            AreaSymbol::GiganticBoulder => 38,
            AreaSymbol::BoulderField => 41,
            AreaSymbol::DenseBoulderField => 44,
            AreaSymbol::StonyGroundSlow => 45,
            AreaSymbol::StonyGroundWalk => 47,
            AreaSymbol::StonyGroundFight => 48,
            AreaSymbol::SandyGround => 49,
            AreaSymbol::BareRock => 50,
            AreaSymbol::UncrossableWaterWithBankLine => 52,
            AreaSymbol::UncrossableWaterWithoutBankLine => 53,
            AreaSymbol::UncrossableWaterDominantWithBankLine => 54,
            AreaSymbol::UncrossableWaterDominantWithoutBankLine => 55,
            AreaSymbol::ShallowWaterWithSolidBankLine => 57,
            AreaSymbol::ShallowWaterWithDashedBankLine => 58,
            AreaSymbol::ShallowWaterWithoutBankLine => 59,
            AreaSymbol::SmallShallowWater => 62,
            AreaSymbol::UncrossableMarshWithBankLine => 67,
            AreaSymbol::UncrossableMarshWithoutBankLine => 68,
            AreaSymbol::Marsh => 69,
            AreaSymbol::IndistinctMarsh => 72,
            AreaSymbol::OpenLand => 77,
            AreaSymbol::OpenLandScatteredTrees => 78,
            AreaSymbol::OpenLandScatteredBushes => 79,
            AreaSymbol::RoughOpenLand => 80,
            AreaSymbol::RoughOpenLandScatteredTrees => 81,
            AreaSymbol::RoughOpenLandScatteredBushes => 82,
            AreaSymbol::Forest => 83,
            AreaSymbol::LightGreen => 84,
            AreaSymbol::LightGreenOneDirectionWhite => 85,
            AreaSymbol::UnderGrowth => 86,
            AreaSymbol::MediumGreen => 87,
            AreaSymbol::MediumGreenOneDirectionWhite => 88,
            AreaSymbol::MediumGreenOneDirectionLightGreen => 89,
            AreaSymbol::DenseUnderGrowth => 90,
            AreaSymbol::DarkGreen => 91,
            AreaSymbol::DarkGreenOneDirectionWhite => 92,
            AreaSymbol::DarkGreenOneDirectionLightGreen => 93,
            AreaSymbol::DarkGreenOneDirectionMediumGreen => 94,
            AreaSymbol::CultivatedLand => 96,
            AreaSymbol::Orchard => 97,
            AreaSymbol::RoughOrchard => 98,
            AreaSymbol::Vineyard => 99,
            AreaSymbol::RoughVineyard => 100,
            AreaSymbol::PavedAreaWithBoundary => 107,
            AreaSymbol::PavedAreaWithoutBoundary => 108,
            AreaSymbol::PrivateArea => 139,
            AreaSymbol::Building => 141,
            AreaSymbol::LargeBuildingWithOutline => 143,
            AreaSymbol::LargeBuildingWithoutOutline => 144,
            AreaSymbol::CanopyWithOutline => 146,
            AreaSymbol::CanopyWithoutOutline => 147,
            AreaSymbol::MagneticNorthBlack => 160,
            AreaSymbol::MagneticNorthBlue => 161,
            AreaSymbol::OutOfBounds => 167,
        }
    }

    fn is_rotateable(&self) -> bool {
        matches!(
            self,
            AreaSymbol::DarkGreenOneDirectionMediumGreen
                | AreaSymbol::DarkGreenOneDirectionLightGreen
                | AreaSymbol::DarkGreenOneDirectionWhite
                | AreaSymbol::MediumGreenOneDirectionLightGreen
                | AreaSymbol::MediumGreenOneDirectionWhite
                | AreaSymbol::LightGreenOneDirectionWhite
                | AreaSymbol::RoughVineyard
                | AreaSymbol::Vineyard
        )
    }
}

impl SymbolTrait for LineSymbol {
    // in meters
    fn min_size(&self, scale: Scale) -> f64 {
        match scale {
            Scale::S10_000 => match self {
                LineSymbol::Contour => todo!(),
                LineSymbol::BasemapContour => todo!(),
                LineSymbol::NegBasemapContour => todo!(),
                LineSymbol::IndexContour => todo!(),
                LineSymbol::Formline => todo!(),
                LineSymbol::EarthBank => todo!(),
                LineSymbol::EarthBankTopLine => todo!(),
                LineSymbol::EarthBankTagLine => todo!(),
                LineSymbol::EarthWall => todo!(),
                LineSymbol::RetainingEarthWall => todo!(),
                LineSymbol::RuinedEarthWall => todo!(),
                LineSymbol::ErosionGully => todo!(),
                LineSymbol::SmallErosionGully => todo!(),
                LineSymbol::ImpassableCliff => todo!(),
                LineSymbol::ImpassableCliffTopLine => todo!(),
                LineSymbol::ImpassableCliffTagLine => todo!(),
                LineSymbol::Cliff => todo!(),
                LineSymbol::CliffWithTags => todo!(),
                LineSymbol::Trench => todo!(),
                LineSymbol::BankLine => todo!(),
                LineSymbol::ShallowWaterOutline => todo!(),
                LineSymbol::ShallowWaterDashedOutline => todo!(),
                LineSymbol::CrossableWatercourse => todo!(),
                LineSymbol::SmallCrossableWatercourse => todo!(),
                LineSymbol::SeasonalWatercourse => todo!(),
                LineSymbol::NarrowMarsh => todo!(),
                LineSymbol::Hedge => todo!(),
                LineSymbol::DistinctCultivationBoundary => todo!(),
                LineSymbol::DistinctVegetationBoundary => todo!(),
                LineSymbol::PavedAreaBoundingLine => todo!(),
                LineSymbol::Road => todo!(),
                LineSymbol::RoadDualCarriageway => todo!(),
                LineSymbol::GravelRoad => todo!(),
                LineSymbol::VehicleTrack => todo!(),
                LineSymbol::Footpath => todo!(),
                LineSymbol::SmallFootpath => todo!(),
                LineSymbol::IndistinctFootpath => todo!(),
                LineSymbol::NarrowRide => todo!(),
                LineSymbol::NarrowRideEasyRunning => todo!(),
                LineSymbol::NarrowRideNormalRunning => todo!(),
                LineSymbol::NarrowRideSlowRunning => todo!(),
                LineSymbol::NarrowRideWalk => todo!(),
                LineSymbol::Railway => todo!(),
                LineSymbol::ImpassableRailway => todo!(),
                LineSymbol::SmallPowerline => todo!(),
                LineSymbol::Powerline => todo!(),
                LineSymbol::MajorPowerline => todo!(),
                LineSymbol::MajorPowerlineWithPylons => todo!(),
                LineSymbol::BridgeTunnel => todo!(),
                LineSymbol::Wall => todo!(),
                LineSymbol::RetainingWall => todo!(),
                LineSymbol::RuinedWall => todo!(),
                LineSymbol::ImpassableWall => todo!(),
                LineSymbol::Fence => todo!(),
                LineSymbol::RuinedFence => todo!(),
                LineSymbol::ImpassableFence => todo!(),
                LineSymbol::PrivateAreaBoundingLine => todo!(),
                LineSymbol::LargeBuildingOutline => todo!(),
                LineSymbol::CanopyOutline => todo!(),
                LineSymbol::Ruin => todo!(),
                LineSymbol::ProminentLinearFeature => todo!(),
                LineSymbol::ImpassableProminentLinearFeature => todo!(),
                LineSymbol::Stairway => todo!(),
                LineSymbol::SimpleOrienteeringCourse => todo!(),
            },
            Scale::S15_000 => match self {
                LineSymbol::Contour => todo!(),
                LineSymbol::BasemapContour => todo!(),
                LineSymbol::NegBasemapContour => todo!(),
                LineSymbol::IndexContour => todo!(),
                LineSymbol::Formline => todo!(),
                LineSymbol::EarthBank => todo!(),
                LineSymbol::EarthBankTopLine => todo!(),
                LineSymbol::EarthBankTagLine => todo!(),
                LineSymbol::EarthWall => todo!(),
                LineSymbol::RetainingEarthWall => todo!(),
                LineSymbol::RuinedEarthWall => todo!(),
                LineSymbol::ErosionGully => todo!(),
                LineSymbol::SmallErosionGully => todo!(),
                LineSymbol::ImpassableCliff => todo!(),
                LineSymbol::ImpassableCliffTopLine => todo!(),
                LineSymbol::ImpassableCliffTagLine => todo!(),
                LineSymbol::Cliff => todo!(),
                LineSymbol::CliffWithTags => todo!(),
                LineSymbol::Trench => todo!(),
                LineSymbol::BankLine => todo!(),
                LineSymbol::ShallowWaterOutline => todo!(),
                LineSymbol::ShallowWaterDashedOutline => todo!(),
                LineSymbol::CrossableWatercourse => todo!(),
                LineSymbol::SmallCrossableWatercourse => todo!(),
                LineSymbol::SeasonalWatercourse => todo!(),
                LineSymbol::NarrowMarsh => todo!(),
                LineSymbol::Hedge => todo!(),
                LineSymbol::DistinctCultivationBoundary => todo!(),
                LineSymbol::DistinctVegetationBoundary => todo!(),
                LineSymbol::PavedAreaBoundingLine => todo!(),
                LineSymbol::Road => todo!(),
                LineSymbol::RoadDualCarriageway => todo!(),
                LineSymbol::GravelRoad => todo!(),
                LineSymbol::VehicleTrack => todo!(),
                LineSymbol::Footpath => todo!(),
                LineSymbol::SmallFootpath => todo!(),
                LineSymbol::IndistinctFootpath => todo!(),
                LineSymbol::NarrowRide => todo!(),
                LineSymbol::NarrowRideEasyRunning => todo!(),
                LineSymbol::NarrowRideNormalRunning => todo!(),
                LineSymbol::NarrowRideSlowRunning => todo!(),
                LineSymbol::NarrowRideWalk => todo!(),
                LineSymbol::Railway => todo!(),
                LineSymbol::ImpassableRailway => todo!(),
                LineSymbol::SmallPowerline => todo!(),
                LineSymbol::Powerline => todo!(),
                LineSymbol::MajorPowerline => todo!(),
                LineSymbol::MajorPowerlineWithPylons => todo!(),
                LineSymbol::BridgeTunnel => todo!(),
                LineSymbol::Wall => todo!(),
                LineSymbol::RetainingWall => todo!(),
                LineSymbol::RuinedWall => todo!(),
                LineSymbol::ImpassableWall => todo!(),
                LineSymbol::Fence => todo!(),
                LineSymbol::RuinedFence => todo!(),
                LineSymbol::ImpassableFence => todo!(),
                LineSymbol::PrivateAreaBoundingLine => todo!(),
                LineSymbol::LargeBuildingOutline => todo!(),
                LineSymbol::CanopyOutline => todo!(),
                LineSymbol::Ruin => todo!(),
                LineSymbol::ProminentLinearFeature => todo!(),
                LineSymbol::ImpassableProminentLinearFeature => todo!(),
                LineSymbol::Stairway => todo!(),
                LineSymbol::SimpleOrienteeringCourse => todo!(),
            },
        }
    }

    fn id(&self) -> u8 {
        match self {
            LineSymbol::Contour => 0,
            LineSymbol::BasemapContour => 2,
            LineSymbol::NegBasemapContour => 3,
            LineSymbol::IndexContour => 4,
            LineSymbol::Formline => 6,
            LineSymbol::EarthBank => 8,
            LineSymbol::EarthBankTopLine => 10,
            LineSymbol::EarthBankTagLine => 11,
            LineSymbol::EarthWall => 12,
            LineSymbol::RetainingEarthWall => 13,
            LineSymbol::RuinedEarthWall => 14,
            LineSymbol::ErosionGully => 15,
            LineSymbol::SmallErosionGully => 16,
            LineSymbol::ImpassableCliff => 25,
            LineSymbol::ImpassableCliffTopLine => 27,
            LineSymbol::ImpassableCliffTagLine => 28,
            LineSymbol::Cliff => 29,
            LineSymbol::CliffWithTags => 31,
            LineSymbol::Trench => 51,
            LineSymbol::BankLine => 56,
            LineSymbol::ShallowWaterOutline => 60,
            LineSymbol::ShallowWaterDashedOutline => 61,
            LineSymbol::CrossableWatercourse => 64,
            LineSymbol::SmallCrossableWatercourse => 65,
            LineSymbol::SeasonalWatercourse => 66,
            LineSymbol::NarrowMarsh => 71,
            LineSymbol::Hedge => 95,
            LineSymbol::DistinctCultivationBoundary => 101,
            LineSymbol::DistinctVegetationBoundary => 102,
            LineSymbol::PavedAreaBoundingLine => 109,
            LineSymbol::Road => 110,
            LineSymbol::RoadDualCarriageway => 111,
            LineSymbol::GravelRoad => 112,
            LineSymbol::VehicleTrack => 113,
            LineSymbol::Footpath => 114,
            LineSymbol::SmallFootpath => 115,
            LineSymbol::IndistinctFootpath => 116,
            LineSymbol::NarrowRide => 117,
            LineSymbol::NarrowRideEasyRunning => 118,
            LineSymbol::NarrowRideNormalRunning => 119,
            LineSymbol::NarrowRideSlowRunning => 120,
            LineSymbol::NarrowRideWalk => 121,
            LineSymbol::Railway => 122,
            LineSymbol::ImpassableRailway => 123,
            LineSymbol::SmallPowerline => 124,
            LineSymbol::Powerline => 125,
            LineSymbol::MajorPowerline => 126,
            LineSymbol::MajorPowerlineWithPylons => 127,
            LineSymbol::BridgeTunnel => 128,
            LineSymbol::Wall => 131,
            LineSymbol::RetainingWall => 132,
            LineSymbol::RuinedWall => 133,
            LineSymbol::ImpassableWall => 134,
            LineSymbol::Fence => 135,
            LineSymbol::RuinedFence => 136,
            LineSymbol::ImpassableFence => 137,
            LineSymbol::PrivateAreaBoundingLine => 140,
            LineSymbol::LargeBuildingOutline => 145,
            LineSymbol::CanopyOutline => 148,
            LineSymbol::Ruin => 149,
            LineSymbol::ProminentLinearFeature => 155,
            LineSymbol::ImpassableProminentLinearFeature => 156,
            LineSymbol::Stairway => 159,
            LineSymbol::SimpleOrienteeringCourse => 166,
        }
    }
}

impl SymbolTrait for PointSymbol {
    fn id(&self) -> u8 {
        match self {
            PointSymbol::SlopelineContour => 1,
            PointSymbol::SlopelineFormline => 7,
            PointSymbol::MinimumEarthBank => 9,
            PointSymbol::DotKnoll => 17,
            PointSymbol::ElongatedDotKnoll => 18,
            PointSymbol::UDepression => 19,
            PointSymbol::Pit => 20,
            PointSymbol::BrokenGroundSingleDot => 22,
            PointSymbol::ProminentLandFeature => 24,
            PointSymbol::MinimumImpassableCliff => 26,
            PointSymbol::MinimumCliff => 30,
            PointSymbol::MinimumCliffWithTags => 32,
            PointSymbol::RockyPitCave => 33,
            PointSymbol::DangerousPit => 34,
            PointSymbol::SmallBoulder => 35,
            PointSymbol::MediumBoulder => 36,
            PointSymbol::LargeBoulder => 37,
            PointSymbol::BoulderCluster => 39,
            PointSymbol::LargeBoulderCluster => 40,
            PointSymbol::BoulderFieldSingleTriangle => 42,
            PointSymbol::BoulderFieldSingleTriangleLarge => 43,
            PointSymbol::StonyGroundSingleDot => 46,
            PointSymbol::Waterhole => 63,
            PointSymbol::MinimumMarsh => 70,
            PointSymbol::MinimumIndistinctMarsh => 73,
            PointSymbol::Well => 74,
            PointSymbol::Spring => 75,
            PointSymbol::ProminentWaterFeature => 76,
            PointSymbol::ProminentTree => 104,
            PointSymbol::ProminentBush => 105,
            PointSymbol::ProminentVegetationFeature => 106,
            PointSymbol::MinimumBridgeTunnel => 129,
            PointSymbol::Footbridge => 130,
            PointSymbol::FenceCrossingPoint => 138,
            PointSymbol::MinimumBuilding => 142,
            PointSymbol::MinimumRuin => 150,
            PointSymbol::HighTower => 151,
            PointSymbol::Tower => 152,
            PointSymbol::Cairn => 153,
            PointSymbol::FodderRack => 154,
            PointSymbol::ProminentManMadeFeatureO => 157,
            PointSymbol::ProminentManMadeFeatureX => 158,
            PointSymbol::RegistrationMark => 162,
            PointSymbol::SpotHeight => 163,
            PointSymbol::OpenOrienteeringMapperLogo => 168,
        }
    }

    fn is_rotateable(&self) -> bool {
        matches!(
            self,
            PointSymbol::BoulderFieldSingleTriangle
                | PointSymbol::BoulderFieldSingleTriangleLarge
                | PointSymbol::ElongatedDotKnoll
                | PointSymbol::FenceCrossingPoint
                | PointSymbol::Footbridge
                | PointSymbol::MinimumBridgeTunnel
                | PointSymbol::MinimumBuilding
                | PointSymbol::MinimumCliff
                | PointSymbol::MinimumCliffWithTags
                | PointSymbol::MinimumEarthBank
                | PointSymbol::MinimumImpassableCliff
                | PointSymbol::MinimumRuin
                | PointSymbol::RockyPitCave
                | PointSymbol::SlopelineContour
                | PointSymbol::SlopelineFormline
                | PointSymbol::Spring
        )
    }
}

impl SymbolTrait for TextSymbol {
    fn id(&self) -> u8 {
        match self {
            TextSymbol::ContourValue => 5,
            TextSymbol::SpotHeight => 164,
            TextSymbol::ControlNumber => 165,
        }
    }
}
