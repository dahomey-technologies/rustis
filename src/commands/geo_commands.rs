use crate::{
    cmd,
    resp::{BulkString, FromSingleValueArray, FromValue, Value},
    ArgsOrCollection, CommandArgs, CommandResult, Error, IntoArgs, PrepareCommand, Result,
    SingleArgOrCollection,
};

/// A group of Redis commands related to Geospatial indices
///
/// # See Also
/// [Redis Geospatial Commands](https://redis.io/commands/?group=geo)
pub trait GeoCommands<T>: PrepareCommand<T> {
    /// Adds the specified geospatial items (longitude, latitude, name) to the specified key.
    ///
    /// # Return
    /// * When used without optional arguments, the number of elements added to the sorted set (excluding score updates).
    /// * If the CH option is specified, the number of elements that were changed (added or updated).
    ///
    /// # See Also
    /// [https://redis.io/commands/geoadd/](https://redis.io/commands/geoadd/)
    #[must_use]
    fn geoadd<K, M, I>(
        &self,
        key: K,
        condition: GeoAddCondition,
        change: bool,
        items: I,
    ) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        I: ArgsOrCollection<(f64, f64, M)>,
    {
        self.prepare_command(
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
    #[must_use]
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
        self.prepare_command(cmd("GEODIST").arg(key).arg(member1).arg(member2).arg(unit))
    }

    /// Return valid [Geohash](https://en.wikipedia.org/wiki/Geohash) strings representing the position of one or more elements
    /// in a sorted set value representing a geospatial index (where elements were added using [geoadd](crate::GeoCommands::geoadd)).
    ///
    /// # Return
    /// An array where each element is the Geohash corresponding to each member name passed as argument to the command.
    ///
    /// # See Also
    /// [https://redis.io/commands/geohash/](https://redis.io/commands/geohash/)
    #[must_use]
    fn geohash<K, M, C>(&self, key: K, members: C) -> CommandResult<T, Vec<String>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        C: SingleArgOrCollection<M>,
    {
        self.prepare_command(cmd("GEOHASH").arg(key).arg(members))
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
    #[must_use]
    fn geopos<K, M, C>(&self, key: K, members: C) -> CommandResult<T, Vec<Option<(f64, f64)>>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        C: SingleArgOrCollection<M>,
    {
        self.prepare_command(cmd("GEOPOS").arg(key).arg(members))
    }

    /// Return the members of a sorted set populated with geospatial information using [geoadd](crate::GeoCommands::geoadd),
    /// which are within the borders of the area specified by a given shape.
    ///
    /// # Return
    /// An array of members + additional information depending
    /// on which `with_xyz` options have been selected
    ///
    /// # See Also
    /// [https://redis.io/commands/geosearch/](https://redis.io/commands/geosearch/)
    #[must_use]
    fn geosearch<K, M1, M2, A>(
        &self,
        key: K,
        from: GeoSearchFrom<M1>,
        by: GeoSearchBy,
        options: GeoSearchOptions,
    ) -> CommandResult<T, A>
    where
        K: Into<BulkString>,
        M1: Into<BulkString>,
        M2: FromValue,
        A: FromSingleValueArray<GeoSearchResult<M2>>,
    {
        self.prepare_command(
            cmd("GEOSEARCH")
                .arg(key)
                .arg(from)
                .arg(by)
                .arg(options),
        )
    }

    /// This command is like [geosearch](crate::GeoCommands::geosearch), but stores the result in destination key.
    ///
    /// # Return
    /// the number of elements in the resulting set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/geosearchstore/>](https://redis.io/commands/geosearchstore/)
    #[must_use]
    fn geosearchstore<D, S, M>(
        &self,
        destination: D,
        source: S,
        from: GeoSearchFrom<M>,
        by: GeoSearchBy,
        options: GeoSearchStoreOptions,
    ) -> CommandResult<T, usize>
    where
        D: Into<BulkString>,
        S: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.prepare_command(
            cmd("GEOSEARCHSTORE")
                .arg(destination)
                .arg(source)
                .arg(from)
                .arg(by)
                .arg(options),
        )
    }
}

/// Condition for the [geoadd](crate::GeoCommands::geoadd) command
pub enum GeoAddCondition {
    /// No option
    None,
    /// Don't update already existing elements. Always add new elements.
    NX,
    /// Only update elements that already exist. Never add elements.
    XX,
}

impl Default for GeoAddCondition {
    fn default() -> Self {
        GeoAddCondition::None
    }
}

impl IntoArgs for GeoAddCondition {
    fn into_args(self, args: crate::CommandArgs) -> crate::CommandArgs {
        match self {
            GeoAddCondition::None => args,
            GeoAddCondition::NX => args.arg("NX"),
            GeoAddCondition::XX => args.arg("XX"),
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

impl IntoArgs for GeoUnit {
    fn into_args(self, args: CommandArgs) -> crate::CommandArgs {
        args.arg(match self {
            GeoUnit::Meters => BulkString::Str("m"),
            GeoUnit::Kilometers => BulkString::Str("km"),
            GeoUnit::Miles => BulkString::Str("mi"),
            GeoUnit::Feet => BulkString::Str("ft"),
        })
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

/// Options for the [`geosearch`](crate::GeoCommands::geosearch) command
#[derive(Default)]
pub struct GeoSearchOptions {
    command_args: CommandArgs,
}

impl GeoSearchOptions {
    #[must_use]
    pub fn order(self, order: GeoSearchOrder) -> Self {
        Self {
            command_args: self.command_args.arg(order),
        }
    }

    #[must_use]
    pub fn count(self, count: usize, any: bool) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count).arg_if(any, "ANY"),
        }
    }

    #[must_use]
    pub fn with_coord(self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHCOORD"),
        }
    }
   
    #[must_use]
    pub fn with_dist(self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHDIST"),
        }
    }

    #[must_use]
    pub fn with_hash(self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHHASH"),
        }
    }
}

impl IntoArgs for GeoSearchOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Result of the [`geosearch_with_options`](crate::GeoCommands::geosearch_with_options) command.
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
        match value {
            Value::BulkString(_) => Ok(GeoSearchResult {
                member: value.into()?,
                distance: None,
                geo_hash: None,
                coordinates: None,
            }),
            Value::Array(_) => {
                let values: Vec<Value> = value.into()?;
                let mut it = values.into_iter();
                let mut distance: Option<f64> = None;
                let mut geo_hash: Option<i64> = None;
                let mut coordinates: Option<(f64, f64)> = None;
        
                let member = match it.next() {
                    Some(value) => value.into()?,
                    None => {
                        return Err(Error::Internal("Unexpected geo search result".to_owned()));
                    }
                };
        
                for value in it {
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
            },
            _ => Err(Error::Internal("Unexpected geo search result".to_owned()))
        }
    }
}

/// Options for the [`geosearchstore`](crate::GeoCommands::geosearchstore) command
#[derive(Default)]
pub struct GeoSearchStoreOptions {
    command_args: CommandArgs,
}

impl GeoSearchStoreOptions {
    #[must_use]
    pub fn order(self, order: GeoSearchOrder) -> Self {
        Self {
            command_args: self.command_args.arg(order),
        }
    }

    #[must_use]
    pub fn count(self, count: usize, any: bool) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count).arg_if(any, "ANY"),
        }
    }

    #[must_use]
    pub fn store_dist(self, store_dist: bool) -> Self {
        Self {
            command_args: self.command_args.arg_if(store_dist, "STOREDIST"),
        }
    }
}

impl IntoArgs for GeoSearchStoreOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}
