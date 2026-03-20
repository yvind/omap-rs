# Omap-rs
[![crates.io version](https://img.shields.io/crates/v/omap.svg)](https://crates.io/crates/omap)
[![docs.rs docs](https://docs.rs/omap/badge.svg)](https://docs.rs/omap)  

A library for working with OpenOrienteering Mapper's .omap files.

For writing new files you can either start with a completely empty map `Omap::new` or use one of the provided templates `Omap::default_15_000`, `Omap::default_10_000` or `Omap::default_4_000`.
Or you can start from an already existing file with `Omap::from_path`.

With the `geo_ref`-feature automatic geo-referencing with magnetic north and scale factor calculation is enabled and done with the `omap::GeoRef::initialize` function. \
It is not enabled by default because of the extra dependencies needed (Proj4rs for coordinate projections, WMM for magnetic north calcualtion and Chrono for time as the magnetic north changes over time). Without this feature the georeferencing must be done by hand.

**NB!** if you change any field (or the entire thing) in the map's `geo_referencing`-field then all the map objects projected/geographic positions will change as their coordinates are given in mm-of-paper and remain untouched.
The best practice is to set the map's geo referencing before adding any objects.

`omap::geo_referencing::Transform` provides functions for going back and forth between mm-of-paper and projected coordinates given by map's georeferencing. And is obtained with calling `get_transform` on the map's `geo_referencing`-field.
