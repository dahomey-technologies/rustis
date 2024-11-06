use crate::{
    client::{prepare_command, PreparedCommand},
    resp::{
        cmd, CollectionResponse, CommandArgs, MultipleArgsCollection, PrimitiveResponse, SingleArg,
        SingleArgCollection, ToArgs,
    },
};
use serde::{
    de::{
        self,
        value::{BytesDeserializer, SeqAccessDeserializer},
        DeserializeOwned, Unexpected, Visitor,
    },
    Deserialize, Deserializer,
};
use std::{fmt, marker::PhantomData};

/// A group of Redis commands related to [`Geospatial`](https://redis.io/docs/data-types/geospatial/) indices
///
/// # See Also
/// [Redis Geospatial Commands](https://redis.io/commands/?group=geo)
pub trait GeoCommands<'a> {
    /// Adds the specified geospatial items (longitude, latitude, name) to the specified key.
    ///
    /// # Return
    /// * When used without optional arguments, the number of elements added to the sorted set (excluding score updates).
    /// * If the CH option is specified, the number of elements that were changed (added or updated).
    ///
    /// # See Also
    /// [<https://redis.io/commands/geoadd/>](https://redis.io/commands/geoadd/)
    #[must_use]
    fn geoadd<K, M, I>(
        self,
        key: K,
        condition: GeoAddCondition,
        change: bool,
        items: I,
    ) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        M: SingleArg,
        I: MultipleArgsCollection<(f64, f64, M)>,
    {
        prepare_command(
            self,
            cmd("GEOADD")
                .arg(key)
                .arg(condition)
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
    fn geodist<K, M>(
        self,
        key: K,
        member1: M,
        member2: M,
        unit: GeoUnit,
    ) -> PreparedCommand<'a, Self, Option<f64>>
    where
        Self: Sized,
        K: SingleArg,
        M: SingleArg,
    {
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
    fn geohash<K, M, C>(self, key: K, members: C) -> PreparedCommand<'a, Self, Vec<String>>
    where
        Self: Sized,
        K: SingleArg,
        M: SingleArg,
        C: SingleArgCollection<M>,
    {
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
    fn geopos<K, M, C>(
        self,
        key: K,
        members: C,
    ) -> PreparedCommand<'a, Self, Vec<Option<(f64, f64)>>>
    where
        Self: Sized,
        K: SingleArg,
        M: SingleArg,
        C: SingleArgCollection<M>,
    {
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
    fn geosearch<K, M1, M2, A>(
        self,
        key: K,
        from: GeoSearchFrom<M1>,
        by: GeoSearchBy,
        options: GeoSearchOptions,
    ) -> PreparedCommand<'a, Self, A>
    where
        Self: Sized,
        K: SingleArg,
        M1: SingleArg,
        M2: PrimitiveResponse + DeserializeOwned,
        A: CollectionResponse<GeoSearchResult<M2>> + DeserializeOwned,
    {
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
    fn geosearchstore<D, S, M>(
        self,
        destination: D,
        source: S,
        from: GeoSearchFrom<M>,
        by: GeoSearchBy,
        options: GeoSearchStoreOptions,
    ) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        D: SingleArg,
        S: SingleArg,
        M: SingleArg,
    {
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
#[derive(Default)]
pub enum GeoAddCondition {
    /// No option
    #[default]
    None,
    /// Don't update already existing elements. Always add new elements.
    NX,
    /// Only update elements that already exist. Never add elements.
    XX,
}

impl ToArgs for GeoAddCondition {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            GeoAddCondition::None => {}
            GeoAddCondition::NX => {
                args.arg("NX");
            }
            GeoAddCondition::XX => {
                args.arg("XX");
            }
        }
    }
}

/// Distance Unit
pub enum GeoUnit {
    Meters,
    Kilometers,
    Miles,
    Feet,
}

impl ToArgs for GeoUnit {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            GeoUnit::Meters => "m",
            GeoUnit::Kilometers => "km",
            GeoUnit::Miles => "mi",
            GeoUnit::Feet => "ft",
        });
    }
}

/// The query's center point is provided by one of these mandatory options:
pub enum GeoSearchFrom<M>
where
    M: SingleArg,
{
    /// Use the position of the given existing `member` in the sorted set.
    FromMember { member: M },
    /// Use the given `longitude` and `latitude` position.
    FromLonLat { longitude: f64, latitude: f64 },
}

impl<M> ToArgs for GeoSearchFrom<M>
where
    M: SingleArg,
{
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            GeoSearchFrom::FromMember { member } => args.arg("FROMMEMBER").arg_ref(member),
            GeoSearchFrom::FromLonLat {
                longitude,
                latitude,
            } => args.arg("FROMLONLAT").arg(*longitude).arg(*latitude),
        };
    }
}

/// The query's shape is provided by one of these mandatory options:
pub enum GeoSearchBy {
    /// Search inside circular area according to given `radius` in the specified `unit`.
    ByRadius { radius: f64, unit: GeoUnit },
    /// Search inside an axis-aligned rectangle, determined by `height` and `width` in the specified `unit`.
    ByBox {
        width: f64,
        height: f64,
        unit: GeoUnit,
    },
}

impl ToArgs for GeoSearchBy {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            GeoSearchBy::ByRadius { radius, unit } => {
                args.arg("BYRADIUS").arg_ref(radius).arg_ref(unit)
            }
            GeoSearchBy::ByBox {
                width,
                height,
                unit,
            } => args
                .arg("BYBOX")
                .arg_ref(width)
                .arg_ref(height)
                .arg_ref(unit),
        };
    }
}

/// Matching items are returned unsorted by default.
/// To sort them, use one of the following two options:
pub enum GeoSearchOrder {
    /// Sort returned items from the nearest to the farthest, relative to the center point.
    Asc,
    /// Sort returned items from the farthest to the nearest, relative to the center point.
    Desc,
}

impl ToArgs for GeoSearchOrder {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            GeoSearchOrder::Asc => args.arg("ASC"),
            GeoSearchOrder::Desc => args.arg("DESC"),
        };
    }
}

/// Options for the [`geosearch`](GeoCommands::geosearch) command
#[derive(Default)]
pub struct GeoSearchOptions {
    command_args: CommandArgs,
}

impl GeoSearchOptions {
    #[must_use]
    pub fn order(mut self, order: GeoSearchOrder) -> Self {
        Self {
            command_args: self.command_args.arg(order).build(),
        }
    }

    #[must_use]
    pub fn count(mut self, count: usize, any: bool) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("COUNT")
                .arg(count)
                .arg_if(any, "ANY")
                .build(),
        }
    }

    #[must_use]
    pub fn with_coord(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHCOORD").build(),
        }
    }

    #[must_use]
    pub fn with_dist(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHDIST").build(),
        }
    }

    #[must_use]
    pub fn with_hash(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHHASH").build(),
        }
    }
}

impl ToArgs for GeoSearchOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Result of the [`geosearch`](GeoCommands::geosearch) command.
#[derive(Debug)]
pub struct GeoSearchResult<M>
where
    M: PrimitiveResponse,
{
    /// The matched member.
    pub member: M,

    /// The distance of the matched member from the specified center.
    pub distance: Option<f64>,

    /// The geohash integer of the matched member
    pub geo_hash: Option<i64>,

    /// The coordinates (longitude, latitude) of the matched member
    pub coordinates: Option<(f64, f64)>,
}

impl<'de, M> Deserialize<'de> for GeoSearchResult<M>
where
    M: PrimitiveResponse + DeserializeOwned,
{
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

        pub struct GeoSearchResultVisitor<M>
        where
            M: PrimitiveResponse,
        {
            phantom: PhantomData<M>,
        }

        impl<'de, M> Visitor<'de> for GeoSearchResultVisitor<M>
        where
            M: PrimitiveResponse + DeserializeOwned,
        {
            type Value = GeoSearchResult<M>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("GeoSearchResult<M>")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let Some(member) = seq.next_element::<M>().map_err(de::Error::custom)? else {
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
                let member = M::deserialize(BytesDeserializer::new(v))?;

                Ok(GeoSearchResult {
                    member,
                    distance: None,
                    geo_hash: None,
                    coordinates: None,
                })
            }
        }

        deserializer.deserialize_any(GeoSearchResultVisitor::<M> {
            phantom: PhantomData,
        })
    }
}

/// Options for the [`geosearchstore`](GeoCommands::geosearchstore) command
#[derive(Default)]
pub struct GeoSearchStoreOptions {
    command_args: CommandArgs,
}

impl GeoSearchStoreOptions {
    #[must_use]
    pub fn order(mut self, order: GeoSearchOrder) -> Self {
        Self {
            command_args: self.command_args.arg(order).build(),
        }
    }

    #[must_use]
    pub fn count(mut self, count: usize, any: bool) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("COUNT")
                .arg(count)
                .arg_if(any, "ANY")
                .build(),
        }
    }

    #[must_use]
    pub fn store_dist(mut self, store_dist: bool) -> Self {
        Self {
            command_args: self.command_args.arg_if(store_dist, "STOREDIST").build(),
        }
    }
}

impl ToArgs for GeoSearchStoreOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}
