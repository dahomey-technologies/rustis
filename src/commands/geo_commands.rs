use crate::{
    cmd,
    resp::{BulkString, FromSingleValueArray, FromValue, Value},
    ArgsOrCollection, CommandResult, Error, IntoArgs, IntoCommandResult, Result,
    SingleArgOrCollection,
};

/// A group of Redis commands related to Geospatial indices
///
/// # See Also
/// [Redis Geospatial Commands](https://redis.io/commands/?group=geo)
pub trait GeoCommands<T>: IntoCommandResult<T> {
    /// Adds the specified geospatial items (longitude, latitude, name) to the specified key.
    ///
    /// # Return
    /// * When used without optional arguments, the number of elements added to the sorted set (excluding score updates).
    /// * If the CH option is specified, the number of elements that were changed (added or updated).
    ///
    /// # See Also
    /// [https://redis.io/commands/geoadd/](https://redis.io/commands/geoadd/)
    fn geoadd<K, M, I>(
        &self,
        key: K,
        condition: Option<GeoAddCondition>,
        change: bool,
        items: I,
    ) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        I: ArgsOrCollection<(f64, f64, M)>,
    {
        self.into_command_result(
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
    /// [https://redis.io/commands/geodist/](https://redis.io/commands/geodist/)
    fn geodist<K, M>(
        &self,
        key: K,
        member1: M,
        member2: M,
        unit: GeoUnit,
    ) -> CommandResult<T, Option<f64>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.into_command_result(cmd("GEODIST").arg(key).arg(member1).arg(member2).arg(unit))
    }

    /// Return valid [Geohash](https://en.wikipedia.org/wiki/Geohash) strings representing the position of one or more elements
    /// in a sorted set value representing a geospatial index (where elements were added using [geoadd](crate::GeoCommands::geoadd)).
    ///
    /// # Return
    /// An array where each element is the Geohash corresponding to each member name passed as argument to the command.
    ///
    /// # See Also
    /// [https://redis.io/commands/geohash/](https://redis.io/commands/geohash/)
    fn geohash<K, M, C>(&self, key: K, members: C) -> CommandResult<T, Vec<String>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        C: SingleArgOrCollection<M>,
    {
        self.into_command_result(cmd("GEOHASH").arg(key).arg(members))
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
    /// [https://redis.io/commands/geopos/](https://redis.io/commands/geopos/)
    fn geopos<K, M, C>(&self, key: K, members: C) -> CommandResult<T, Vec<Option<(f64, f64)>>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        C: SingleArgOrCollection<M>,
    {
        self.into_command_result(cmd("GEOPOS").arg(key).arg(members))
    }

    /// Return the members of a sorted set populated with geospatial information using [geoadd](crate::GeoCommands::geoadd),
    /// which are within the borders of the area specified by a given shape.
    ///
    /// # Return
    /// An array of members
    ///
    /// # See Also
    /// [https://redis.io/commands/geosearch/](https://redis.io/commands/geosearch/)
    fn geosearch<K, M1, M2, A>(
        &self,
        key: K,
        from: GeoSearchFrom<M1>,
        by: GeoSearchBy,
        order: Option<GeoSearchOrder>,
        count: Option<(usize, bool)>,
    ) -> CommandResult<T, A>
    where
        K: Into<BulkString>,
        M1: Into<BulkString>,
        M2: FromValue,
        A: FromSingleValueArray<M2>,
    {
        self.into_command_result(cmd("GEOSEARCH").arg(key).arg(from).arg(by).arg(order).arg(
            count.map(|(count, any)| {
                if any {
                    (count, Some("ANY"))
                } else {
                    (count, None)
                }
            }),
        ))
    }

    /// Return the members of a sorted set populated with geospatial information using [geoadd](crate::GeoCommands::geoadd),
    /// which are within the borders of the area specified by a given shape.
    ///
    /// # Return
    /// An array of members + additional information depending
    /// on which with_xyz options have been selected
    ///
    /// # See Also
    /// [https://redis.io/commands/geosearch/](https://redis.io/commands/geosearch/)
    fn geosearch_with_options<K, M1, M2, A>(
        &self,
        key: K,
        from: GeoSearchFrom<M1>,
        by: GeoSearchBy,
        order: Option<GeoSearchOrder>,
        count: Option<(usize, bool)>,
        with_coord: bool,
        with_dist: bool,
        with_hash: bool,
    ) -> CommandResult<T, A>
    where
        K: Into<BulkString>,
        M1: Into<BulkString>,
        M2: FromValue,
        A: FromSingleValueArray<GeoSearchResult<M2>>,
    {
        self.into_command_result(
            cmd("GEOSEARCH")
                .arg(key)
                .arg(from)
                .arg(by)
                .arg(order)
                .arg(count.map(|(count, any)| {
                    if any {
                        ("COUNT", count, Some("ANY"))
                    } else {
                        ("COUNT", count, None)
                    }
                }))
                .arg_if(with_coord, "WITHCOORD")
                .arg_if(with_dist, "WITHDIST")
                .arg_if(with_hash, "WITHHASH"),
        )
    }

    /// This command is like [geosearch](crate::GeoCommands::geosearch), but stores the result in destination key.
    ///
    /// # Return
    /// the number of elements in the resulting set.
    ///
    /// # See Also
    /// [https://redis.io/commands/geosearchstore/](https://redis.io/commands/geosearchstore/)
    fn geosearchstore<D, S, M>(
        &self,
        destination: D,
        source: S,
        from: GeoSearchFrom<M>,
        by: GeoSearchBy,
        order: Option<GeoSearchOrder>,
        count: Option<(usize, bool)>,
        store_dist: bool,
    ) -> CommandResult<T, usize>
    where
        D: Into<BulkString>,
        S: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.into_command_result(
            cmd("GEOSEARCHSTORE")
                .arg(destination)
                .arg(source)
                .arg(from)
                .arg(by)
                .arg(order)
                .arg(count.map(|(count, any)| {
                    if any {
                        ("COUNT", count, Some("ANY"))
                    } else {
                        ("COUNT", count, None)
                    }
                }))
                .arg_if(store_dist, "STOREDIST"),
        )
    }
}

/// Condition for the [geoadd](crate::GeoCommands::geoadd) command
pub enum GeoAddCondition {
    /// Don't update already existing elements. Always add new elements.
    NX,
    /// Only update elements that already exist. Never add elements.
    XX,
}

impl From<GeoAddCondition> for BulkString {
    fn from(cond: GeoAddCondition) -> Self {
        match cond {
            GeoAddCondition::NX => BulkString::Str("NX"),
            GeoAddCondition::XX => BulkString::Str("XX"),
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

impl From<GeoUnit> for BulkString {
    fn from(unit: GeoUnit) -> Self {
        match unit {
            GeoUnit::Meters => BulkString::Str("m"),
            GeoUnit::Kilometers => BulkString::Str("km"),
            GeoUnit::Miles => BulkString::Str("mi"),
            GeoUnit::Feet => BulkString::Str("ft"),
        }
    }
}

/// The query's center point is provided by one of these mandatory options:
pub enum GeoSearchFrom<M>
where
    M: Into<BulkString>,
{
    /// Use the position of the given existing `member` in the sorted set.
    FromMember { member: M },
    /// Use the given `longitude` and `latitude` position.
    FromLonLat { longitude: f64, latitude: f64 },
}

impl<M> IntoArgs for GeoSearchFrom<M>
where
    M: Into<BulkString>,
{
    fn into_args(self, args: crate::CommandArgs) -> crate::CommandArgs {
        match self {
            GeoSearchFrom::FromMember { member } => args.arg("FROMMEMBER").arg(member),
            GeoSearchFrom::FromLonLat {
                longitude,
                latitude,
            } => args.arg("FROMLONLAT").arg(longitude).arg(latitude),
        }
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

impl IntoArgs for GeoSearchBy {
    fn into_args(self, args: crate::CommandArgs) -> crate::CommandArgs {
        match self {
            GeoSearchBy::ByRadius { radius, unit } => args.arg("BYRADIUS").arg(radius).arg(unit),
            GeoSearchBy::ByBox {
                width,
                height,
                unit,
            } => args.arg("BYBOX").arg(width).arg(height).arg(unit),
        }
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

impl IntoArgs for GeoSearchOrder {
    fn into_args(self, args: crate::CommandArgs) -> crate::CommandArgs {
        match self {
            GeoSearchOrder::Asc => args.arg("ASC"),
            GeoSearchOrder::Desc => args.arg("DESC"),
        }
    }
}

/// Result of a GeoSearch with options
#[derive(Debug)]
pub struct GeoSearchResult<M>
where
    M: FromValue,
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

impl<M> FromValue for GeoSearchResult<M>
where
    M: FromValue,
{
    fn from_value(value: Value) -> Result<Self> {
        let values: Vec<Value> = value.into()?;
        let mut it = values.into_iter();
        let member: M;
        let mut distance: Option<f64> = None;
        let mut geo_hash: Option<i64> = None;
        let mut coordinates: Option<(f64, f64)> = None;

        match it.next() {
            Some(value) => member = value.into()?,
            None => {
                return Err(Error::Internal("Unexpected geo search result".to_owned()));
            }
        }

        while let Some(value) = it.next() {
            match value {
                Value::BulkString(BulkString::Binary(_)) => distance = Some(value.into()?),
                Value::Integer(h) => geo_hash = Some(h),
                Value::Array(_) => coordinates = Some(value.into()?),
                _ => return Err(Error::Internal("Unexpected geo search result".to_owned())),
            }
        }

        Ok(GeoSearchResult {
            member,
            distance,
            geo_hash,
            coordinates,
        })
    }
}
