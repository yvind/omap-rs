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
    FormLine,
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
    SmallPowerLine,
    LargePowerLine,
    MajorPowerLine,
    MajorPowerLineWithPylons,
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
    SlopeLineContour,
    SlopeLineFormLine,
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

    /// some symbols are rotatable or their pattern are rotatable
    fn is_rotatable(&self) -> bool {
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

    fn is_rotatable(&self) -> bool {
        match self {
            Symbol::Area(a) => a.is_rotatable(),
            Symbol::Line(l) => l.is_rotatable(),
            Symbol::Point(p) => p.is_rotatable(),
            Symbol::Text(t) => t.is_rotatable(),
        }
    }
}

impl SymbolTrait for AreaSymbol {
    // in square meters
    fn min_size(&self, scale: Scale) -> f64 {
        const CONVERSION: f64 = 4. / 9.;

        let min = match self {
            AreaSymbol::BrokenGround => 100.,
            AreaSymbol::VeryBrokenGround => 49.,
            AreaSymbol::GiganticBoulder => 67.,
            AreaSymbol::BoulderField => 225.,
            AreaSymbol::DenseBoulderField => 100.,
            AreaSymbol::StonyGroundSlow => 100.,
            AreaSymbol::StonyGroundWalk => 64.,
            AreaSymbol::StonyGroundFight => 49.,
            AreaSymbol::SandyGround => 225.,
            AreaSymbol::BareRock => 225.,
            AreaSymbol::UncrossableWaterWithBankLine => 64.,
            AreaSymbol::UncrossableWaterWithoutBankLine => 64.,
            AreaSymbol::UncrossableWaterDominantWithBankLine => 64.,
            AreaSymbol::UncrossableWaterDominantWithoutBankLine => 64.,
            AreaSymbol::ShallowWaterWithSolidBankLine => 110.25,
            AreaSymbol::ShallowWaterWithDashedBankLine => 110.25,
            AreaSymbol::ShallowWaterWithoutBankLine => 110.25,
            AreaSymbol::SmallShallowWater => 64.,
            AreaSymbol::UncrossableMarshWithBankLine => 110.25,
            AreaSymbol::UncrossableMarshWithoutBankLine => 110.25,
            AreaSymbol::Marsh => 45.,
            AreaSymbol::IndistinctMarsh => 315.,
            AreaSymbol::OpenLand => 64.,
            AreaSymbol::OpenLandScatteredTrees => 900.,
            AreaSymbol::OpenLandScatteredBushes => 900.,
            AreaSymbol::RoughOpenLand => 225.,
            AreaSymbol::RoughOpenLandScatteredTrees => 1406.25,
            AreaSymbol::RoughOpenLandScatteredBushes => 1406.25,
            AreaSymbol::LightGreen => 225.,
            AreaSymbol::LightGreenOneDirectionWhite => 225.,
            AreaSymbol::UnderGrowth => 337.5,
            AreaSymbol::MediumGreen => 110.25,
            AreaSymbol::MediumGreenOneDirectionWhite => 110.25,
            AreaSymbol::MediumGreenOneDirectionLightGreen => 110.25,
            AreaSymbol::DenseUnderGrowth => 225.,
            AreaSymbol::DarkGreen => 64.,
            AreaSymbol::DarkGreenOneDirectionWhite => 64.,
            AreaSymbol::DarkGreenOneDirectionLightGreen => 64.,
            AreaSymbol::DarkGreenOneDirectionMediumGreen => 64.,
            AreaSymbol::CultivatedLand => 2025.,
            AreaSymbol::Orchard => 900.,
            AreaSymbol::RoughOrchard => 900.,
            AreaSymbol::Vineyard => 900.,
            AreaSymbol::RoughVineyard => 900.,
            AreaSymbol::PavedAreaWithBoundary => 225.,
            AreaSymbol::PavedAreaWithoutBoundary => 225.,
            AreaSymbol::PrivateArea => 225.,
            AreaSymbol::Building => 56.25,
            AreaSymbol::LargeBuildingWithOutline => 5625.,
            AreaSymbol::LargeBuildingWithoutOutline => 5625.,
            AreaSymbol::CanopyWithOutline => 81.,
            AreaSymbol::CanopyWithoutOutline => 81.,
            AreaSymbol::OutOfBounds => 2025.,
            _ => 0.,
        };
        match scale {
            Scale::S10_000 => min * CONVERSION,
            Scale::S15_000 => min,
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

    fn is_rotatable(&self) -> bool {
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
        const CONVERSION: f64 = 2. / 3.;
        let min = match self {
            LineSymbol::FormLine => 16.5,
            LineSymbol::EarthBank => 9.,
            LineSymbol::EarthBankTopLine => 9.,
            LineSymbol::EarthBankTagLine => 6.,
            LineSymbol::EarthWall => 21.,
            LineSymbol::RetainingEarthWall => 21.,
            LineSymbol::RuinedEarthWall => 55.,
            LineSymbol::ErosionGully => 17.25,
            LineSymbol::SmallErosionGully => 10.5,
            LineSymbol::ImpassableCliff => 9.,
            LineSymbol::ImpassableCliffTopLine => 9.,
            LineSymbol::ImpassableCliffTagLine => 6.,
            LineSymbol::Cliff => 9.,
            LineSymbol::CliffWithTags => 9.,
            LineSymbol::Trench => 15.,
            LineSymbol::CrossableWatercourse => 15.,
            LineSymbol::SmallCrossableWatercourse => 15.,
            LineSymbol::SeasonalWatercourse => 41.,
            LineSymbol::NarrowMarsh => 10.5,
            LineSymbol::Hedge => 17.,
            LineSymbol::DistinctCultivationBoundary => 30.,
            LineSymbol::DistinctVegetationBoundary => 27.,
            LineSymbol::VehicleTrack => 94.,
            LineSymbol::Footpath => 64.,
            LineSymbol::SmallFootpath => 34.,
            LineSymbol::IndistinctFootpath => 79.5,
            LineSymbol::NarrowRide => 48.,
            LineSymbol::NarrowRideEasyRunning => 48.,
            LineSymbol::NarrowRideNormalRunning => 48.,
            LineSymbol::NarrowRideSlowRunning => 48.,
            LineSymbol::NarrowRideWalk => 48.,
            LineSymbol::Railway => 60.,
            LineSymbol::ImpassableRailway => 60.,
            LineSymbol::SmallPowerLine => 75.,
            LineSymbol::LargePowerLine => 75.,
            LineSymbol::MajorPowerLine => 75.,
            LineSymbol::MajorPowerLineWithPylons => 75.,
            LineSymbol::BridgeTunnel => 6.,
            LineSymbol::Wall => 21.,
            LineSymbol::RetainingWall => 21.,
            LineSymbol::RuinedWall => 55.,
            LineSymbol::ImpassableWall => 45.,
            LineSymbol::Fence => 22.5,
            LineSymbol::RuinedFence => 55.,
            LineSymbol::ImpassableFence => 30.,
            LineSymbol::Ruin => 48.,
            LineSymbol::ProminentLinearFeature => 22.5,
            LineSymbol::ImpassableProminentLinearFeature => 30.,
            LineSymbol::Stairway => 24.,
            _ => 0.,
        };

        match scale {
            Scale::S10_000 => min * CONVERSION,
            Scale::S15_000 => min,
        }
    }

    fn id(&self) -> u8 {
        match self {
            LineSymbol::Contour => 0,
            LineSymbol::BasemapContour => 2,
            LineSymbol::NegBasemapContour => 3,
            LineSymbol::IndexContour => 4,
            LineSymbol::FormLine => 6,
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
            LineSymbol::SmallPowerLine => 124,
            LineSymbol::LargePowerLine => 125,
            LineSymbol::MajorPowerLine => 126,
            LineSymbol::MajorPowerLineWithPylons => 127,
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
            PointSymbol::SlopeLineContour => 1,
            PointSymbol::SlopeLineFormLine => 7,
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

    fn is_rotatable(&self) -> bool {
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
                | PointSymbol::SlopeLineContour
                | PointSymbol::SlopeLineFormLine
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
