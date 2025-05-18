use super::SymbolTrait;

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
