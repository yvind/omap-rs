# Omap-rs
A library for writing `geo_types`-geometries to OpenOrienteering Mapper's .omap files.  

The files are automatically georeferenced (including scale factors) and magnetic north aligned (using the current WMM, date and map-location) if a Coordinate Reference System is provided (by EPSG code). 

Scales 1:15_000 and 1:10_000 are supported.