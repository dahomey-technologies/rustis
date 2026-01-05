use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{Response, cmd, serialize_flag},
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{
        self, Unexpected, Visitor,
        value::{BytesDeserializer, SeqAccessDeserializer},
    },
};
use std::{fmt, marker::PhantomData};

/// A group of Redis commands related to [`Geospatial`](https://redis.io/docs/data-types/geospatial/) indices
///
/// # See Also
/// [Redis Geospatial Commands](https://redis.io/commands/?group=geo)
pub trait GeoCommands<'a>: Sized {
    /// Adds the specified geospatial items (longitude, latitude, name) to the specified key.
    ///
    /// # Return
    /// * When used without optional arguments, the number of elements added to the sorted set (excluding score updates).
    /// * If the CH option is specified, the number of elements that were changed (added or updated).
    ///
    /// # See Also
    /// [<https://redis.io/commands/geoadd/>](https://redis.io/commands/geoadd/)
    #[must_use]
    fn geoadd(
        self,
        key: impl Serialize,
        condition: impl Into<Option<GeoAddCondition>>,
        change: bool,
        items: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(
            self,
            cmd("GEOADD")
                .arg(key)
                .arg(condition.into())
                .arg_if(change, "CH")
                .arg(items),
        )
    }

    /// Return the distance between two members in the geospatial index
    /// represented by the sorted set.
    ///
    /// # Return
    /// The distance in the specified unit, or None if one or both the elements are missing.
    ///
    /// # See Also
    /// [<https://redis.io/commands/geodist/>](https://redis.io/commands/geodist/)
    #[must_use]
    fn geodist(
        self,
        key: impl Serialize,
        member1: impl Serialize,
        member2: impl Serialize,
        unit: GeoUnit,
    ) -> PreparedCommand<'a, Self, Option<f64>> {
        prepare_command(
            self,
            cmd("GEODIST").arg(key).arg(member1).arg(member2).arg(unit),
        )
    }

    /// Return valid [Geohash](https://en.wikipedia.org/wiki/Geohash) strings representing the position of one or more elements
    /// in a sorted set value representing a geospatial index (where elements were added using [geoadd](GeoCommands::geoadd)).
    ///
    /// # Return
    /// An array where each element is the Geohash corresponding to each member name passed as argument to the command.
    ///
    /// # See Also
    /// [<https://redis.io/commands/geohash/>](https://redis.io/commands/geohash/)
    #[must_use]
    fn geohash<R: Response>(
        self,
        key: impl Serialize,
        members: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("GEOHASH").arg(key).arg(members))
    }

    /// Return the positions (longitude,latitude) of all the specified members
    ///  of the geospatial index represented by the sorted set at key.
    ///
    /// # Return
    /// n array where each element is a two elements array representing longitude and latitude
    /// (x,y) of each member name passed as argument to the command.
    /// Non existing elements are reported as NULL elements of the array.
    ///
    /// # See Also
    /// [<https://redis.io/commands/geopos/>](https://redis.io/commands/geopos/)
    #[must_use]
    fn geopos(
        self,
        key: impl Serialize,
        members: impl Serialize,
    ) -> PreparedCommand<'a, Self, Vec<Option<(f64, f64)>>> {
        prepare_command(self, cmd("GEOPOS").arg(key).arg(members))
    }

    /// Return the members of a sorted set populated with geospatial information using [geoadd](GeoCommands::geoadd),
    /// which are within the borders of the area specified by a given shape.
    ///
    /// # Return
    /// An array of members + additional information depending
    /// on which `with_xyz` options have been selected
    ///
    /// # See Also
    /// [<https://redis.io/commands/geosearch/>](https://redis.io/commands/geosearch/)
    #[must_use]
    fn geosearch<'b, R: Response>(
        self,
        key: impl Serialize,
        from: GeoSearchFrom<'b>,
        by: GeoSearchBy,
        options: GeoSearchOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("GEOSEARCH").arg(key).arg(from).arg(by).arg(options),
        )
    }

    /// This command is like [geosearch](GeoCommands::geosearch), but stores the result in destination key.
    ///
    /// # Return
    /// the number of elements in the resulting set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/geosearchstore/>](https://redis.io/commands/geosearchstore/)
    #[must_use]
    fn geosearchstore<'b>(
        self,
        destination: impl Serialize,
        source: impl Serialize,
        from: GeoSearchFrom<'b>,
        by: GeoSearchBy,
        options: GeoSearchStoreOptions,
    ) -> PreparedCommand<'a, Self, u32> {
        prepare_command(
            self,
            cmd("GEOSEARCHSTORE")
                .arg(destination)
                .arg(source)
                .arg(from)
                .arg(by)
                .arg(options),
        )
    }
}

/// Condition for the [`geoadd`](GeoCommands::geoadd) command
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum GeoAddCondition {
    /// Don't update already existing elements. Always add new elements.
    NX,
    /// Only update elements that already exist. Never add elements.
    XX,
}

/// Distance Unit
#[derive(Serialize)]
pub enum GeoUnit {
    #[serde(rename = "M")]
    Meters,
    #[serde(rename = "KM")]
    Kilometers,
    #[serde(rename = "MI")]
    Miles,
    #[serde(rename = "FT")]
    Feet,
}

/// The query's center point is provided by one of these mandatory options:
#[derive(Serialize)]
#[serde(rename_all(serialize = "UPPERCASE"))]
pub enum GeoSearchFrom<'a> {
    /// Use the position of the given existing `member` in the sorted set.
    FromMember(&'a str),
    /// Use the given `longitude` and `latitude` position.
    FromLonLat(f64, f64),
}

impl<'a> GeoSearchFrom<'a> {
    pub fn from_member(member: &'a str) -> Self {
        Self::FromMember(member)
    }

    pub fn from_longitude_latitude(longitude: f64, latitude: f64) -> Self {
        Self::FromLonLat(longitude, latitude)
    }
}

/// The query's shape is provided by one of these mandatory options:
#[derive(Serialize)]
#[serde(rename_all(serialize = "UPPERCASE"))]
pub enum GeoSearchBy {
    /// Search inside circular area according to given `radius` in the specified `unit`.
    ByRadius(f64, GeoUnit),
    /// Search inside an axis-aligned rectangle, determined by `height` and `width` in the specified `unit`.
    ByBox(f64, f64, GeoUnit),
}

impl GeoSearchBy {
    pub fn by_radius(radius: f64, unit: GeoUnit) -> Self {
        Self::ByRadius(radius, unit)
    }

    pub fn by_box(width: f64, height: f64, unit: GeoUnit) -> Self {
        Self::ByBox(width, height, unit)
    }
}

/// Matching items are returned unsorted by default.
/// To sort them, use one of the following two options:
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum GeoSearchOrder {
    /// Sort returned items from the nearest to the farthest, relative to the center point.
    Asc,
    /// Sort returned items from the farthest to the nearest, relative to the center point.
    Desc,
}

/// Options for the [`geosearch`](GeoCommands::geosearch) command
#[derive(Default, Serialize)]
#[serde(rename_all(serialize = "UPPERCASE"))]
pub struct GeoSearchOptions {
    #[serde(rename="", skip_serializing_if = "Option::is_none")]
    order: Option<GeoSearchOrder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    any: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withcoord: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withdist: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withhash: bool,
}

impl GeoSearchOptions {
    #[must_use]
    pub fn order(mut self, order: GeoSearchOrder) -> Self {
        self.order = Some(order);
        self
    }

    #[must_use]
    pub fn count(mut self, count: u32, any: bool) -> Self {
        self.count = Some(count);
        self.any = any;
        self
    }

    #[must_use]
    pub fn with_coord(mut self) -> Self {
        self.withcoord = true;
        self
    }

    #[must_use]
    pub fn with_dist(mut self) -> Self {
        self.withdist = true;
        self
    }

    #[must_use]
    pub fn with_hash(mut self) -> Self {
        self.withhash = true;
        self
    }
}

/// Result of the [`geosearch`](GeoCommands::geosearch) command.
#[derive(Debug)]
pub struct GeoSearchResult<R: Response> {
    /// The matched member.
    pub member: R,

    /// The distance of the matched member from the specified center.
    pub distance: Option<f64>,

    /// The geohash integer of the matched member
    pub geo_hash: Option<i64>,

    /// The coordinates (longitude, latitude) of the matched member
    pub coordinates: Option<(f64, f64)>,
}

impl<'de, R: Response + Deserialize<'de>> Deserialize<'de> for GeoSearchResult<R> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        pub enum GeoSearchResultField {
            Distance(f64),
            GeoHash(i64),
            Coordinates((f64, f64)),
        }

        impl<'de> Deserialize<'de> for GeoSearchResultField {
            #[inline]
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct GeoSearchResultFieldVisitor;

                impl<'de> Visitor<'de> for GeoSearchResultFieldVisitor {
                    type Value = GeoSearchResultField;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("GeoSearchResultField")
                    }

                    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        let Ok(distance) = std::str::from_utf8(v) else {
                            return Err(de::Error::invalid_value(
                                Unexpected::Bytes(v),
                                &"A valid f64 encoded in an UTF8 string",
                            ));
                        };

                        let Ok(distance) = distance.parse::<f64>() else {
                            return Err(de::Error::invalid_value(
                                Unexpected::Bytes(v),
                                &"A valid f64 encoded in an UTF8 string",
                            ));
                        };

                        Ok(GeoSearchResultField::Distance(distance))
                    }

                    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Ok(GeoSearchResultField::GeoHash(v))
                    }

                    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
                    where
                        A: de::SeqAccess<'de>,
                    {
                        let coordinates =
                            <(f64, f64)>::deserialize(SeqAccessDeserializer::new(seq))?;
                        Ok(GeoSearchResultField::Coordinates(coordinates))
                    }
                }

                deserializer.deserialize_any(GeoSearchResultFieldVisitor)
            }
        }

        pub struct GeoSearchResultVisitor<R: Response> {
            phantom: PhantomData<R>,
        }

        impl<'de, R: Response + Deserialize<'de>> Visitor<'de> for GeoSearchResultVisitor<R> {
            type Value = GeoSearchResult<R>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("GeoSearchResult<M>")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let Some(member) = seq.next_element::<R>().map_err(de::Error::custom)? else {
                    return Err(de::Error::invalid_length(0, &"more elements in sequence"));
                };

                let mut distance: Option<f64> = None;
                let mut geo_hash: Option<i64> = None;
                let mut coordinates: Option<(f64, f64)> = None;

                while let Some(field) = seq.next_element::<GeoSearchResultField>()? {
                    match field {
                        GeoSearchResultField::Distance(d) => distance = Some(d),
                        GeoSearchResultField::GeoHash(gh) => geo_hash = Some(gh),
                        GeoSearchResultField::Coordinates(c) => coordinates = Some(c),
                    }
                }

                Ok(GeoSearchResult {
                    member,
                    distance,
                    geo_hash,
                    coordinates,
                })
            }

            fn visit_borrowed_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let member = R::deserialize(BytesDeserializer::new(v))?;

                Ok(GeoSearchResult {
                    member,
                    distance: None,
                    geo_hash: None,
                    coordinates: None,
                })
            }
        }

        deserializer.deserialize_any(GeoSearchResultVisitor::<R> {
            phantom: PhantomData,
        })
    }
}

/// Options for the [`geosearchstore`](GeoCommands::geosearchstore) command
#[derive(Default, Serialize)]
#[serde(rename_all(serialize = "UPPERCASE"))]
pub struct GeoSearchStoreOptions {
    #[serde(rename="", skip_serializing_if = "Option::is_none")]
    order: Option<GeoSearchOrder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    any: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    storedist: bool,
}

impl GeoSearchStoreOptions {
    #[must_use]
    pub fn order(mut self, order: GeoSearchOrder) -> Self {
        self.order = Some(order);
        self
    }

    #[must_use]
    pub fn count(mut self, count: u32, any: bool) -> Self {
        self.count = Some(count);
        self.any = any;
        self
    }

    #[must_use]
    pub fn store_dist(mut self, store_dist: bool) -> Self {
        self.storedist = store_dist;
        self
    }
}
