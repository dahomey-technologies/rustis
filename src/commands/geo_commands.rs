use crate::{
    cmd,
    resp::{BulkString, FromSingleValueArray, FromValue, Value},
    ArgsOrCollection, Command, CommandSend, Error, Future, IntoArgs, Result, SingleArgOrCollection,
};

/// A group of Redis commands related to Geospatial indices
///
/// # See Also
/// [Redis Geospatial Commands](https://redis.io/commands/?group=geo)
pub trait GeoCommands: CommandSend {
    /// Adds the specified geospatial items (longitude, latitude, name) to the specified key.
    ///
    /// # Return
    /// GeoAdd builder
    ///
    /// # See Also
    /// [https://redis.io/commands/geoadd/](https://redis.io/commands/geoadd/)
    fn geoadd<K>(&self, key: K) -> GeoAdd<Self>
    where
        K: Into<BulkString>,
    {
        GeoAdd {
            geo_commands: &self,
            cmd: cmd("GEOADD").arg(key),
        }
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
    ) -> Future<'_, Option<f64>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.send_into(cmd("GEODIST").arg(key).arg(member1).arg(member2).arg(unit))
    }

    /// Return valid [Geohash](https://en.wikipedia.org/wiki/Geohash) strings representing the position of one or more elements
    /// in a sorted set value representing a geospatial index (where elements were added using [geoadd](crate::GeoCommands::geoadd)).
    ///
    /// # Return
    /// An array where each element is the Geohash corresponding to each member name passed as argument to the command.
    ///
    /// # See Also
    /// [https://redis.io/commands/geohash/](https://redis.io/commands/geohash/)
    fn geohash<K, M, C>(&self, key: K, members: C) -> Future<'_, Vec<String>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        C: SingleArgOrCollection<M>,
    {
        self.send_into(cmd("GEOHASH").arg(key).arg(members))
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
    fn geopos<K, M, C>(&self, key: K, members: C) -> Future<'_, Vec<Option<(f64, f64)>>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        C: SingleArgOrCollection<M>,
    {
        self.send_into(cmd("GEOPOS").arg(key).arg(members))
    }

    /// Return the members of a sorted set populated with geospatial information using [geoadd](crate::GeoCommands::geoadd),
    /// which are within the borders of the area specified by a given shape.
    ///
    /// # Return
    /// [GeoSearch builder](crate::GeoSearch)
    ///
    /// # See Also
    /// [https://redis.io/commands/geosearch/](https://redis.io/commands/geosearch/)
    fn geosearch<K, M>(&self, key: K, from: GeoSearchFrom<M>, by: GeoSearchBy) -> GeoSearch<Self>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        GeoSearch {
            geo_commands: &self,
            cmd: cmd("GEOSEARCH").arg(key).arg(from).arg(by),
        }
    }

    /// This command is like [geosearch](crate::GeoCommands::geosearch), but stores the result in destination key.
    ///
    /// # Return
    /// [GeoSearchStore builder](crate::GeoSearchStore)
    ///
    /// # See Also
    /// [https://redis.io/commands/geosearchstore/](https://redis.io/commands/geosearchstore/)
    fn geosearchstore<D, S, M>(
        &self,
        destination: D,
        source: S,
        from: GeoSearchFrom<M>,
        by: GeoSearchBy,
    ) -> GeoSearchStore<Self>
    where
        D: Into<BulkString>,
        S: Into<BulkString>,
        M: Into<BulkString>,
    {
        GeoSearchStore {
            geo_commands: &self,
            cmd: cmd("GEOSEARCHSTORE").arg(destination).arg(source).arg(from).arg(by),
        }
    }
}

/// Builder for the [geoadd](crate::GeoCommands::geoadd) command
pub struct GeoAdd<'a, T: GeoCommands + ?Sized> {
    geo_commands: &'a T,
    cmd: Command,
}

impl<'a, T: GeoCommands + ?Sized> GeoAdd<'a, T> {
    /// Don't update already existing elements. Always add new elements.
    pub fn nx(self) -> Self {
        Self {
            geo_commands: self.geo_commands,
            cmd: self.cmd.arg("NX"),
        }
    }

    /// Only update elements that already exist. Never add elements.
    pub fn xx(self) -> Self {
        Self {
            geo_commands: self.geo_commands,
            cmd: self.cmd.arg("XX"),
        }
    }

    /// Modify the return value from the number of new elements added,
    /// to the total number of elements changed (CH is an abbreviation of changed)
    pub fn ch(self) -> Self {
        Self {
            geo_commands: self.geo_commands,
            cmd: self.cmd.arg("CH"),
        }
    }

    /// # Return
    /// * When used without optional arguments, the number of elements added to the sorted set (excluding score updates).
    /// * If the CH option is specified, the number of elements that were changed (added or updated).
    pub fn execute<M, I>(self, items: I) -> Future<'a, usize>
    where
        M: Into<BulkString>,
        I: ArgsOrCollection<(f64, f64, M)>,
    {
        self.geo_commands.send_into(self.cmd.arg(items))
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

/// Builder for the [geosearch](crate::GeoCommands::geosearch) command
pub struct GeoSearch<'a, T: GeoCommands + ?Sized> {
    geo_commands: &'a T,
    cmd: Command,
}

impl<'a, T: GeoCommands + ?Sized> GeoSearch<'a, T> {
    /// Matching items are returned unsorted by default. To sort them, use one of the following two options
    /// * [Asc](crate::GeoSearchOrder::Asc): Sort returned items from the nearest to the farthest, relative to the center point.
    /// * [Desc](crate::GeoSearchOrder::Desc): Sort returned items from the farthest to the nearest, relative to the center point.
    pub fn order(self, order: GeoSearchOrder) -> Self {
        Self {
            geo_commands: self.geo_commands,
            cmd: self.cmd.arg(order),
        }
    }

    /// Limit the results to the first `count` matching items,
    /// When the `any` option is used, the command returns as soon as enough matches are found.
    pub fn count(self, count: usize, any: bool) -> Self {
        let mut cmd = self.cmd.arg("COUNT").arg(count);

        if any {
            cmd = cmd.arg("ANY");
        }

        Self {
            geo_commands: self.geo_commands,
            cmd,
        }
    }

    /// Result without `with` option specified
    /// # Return
    /// A linear array of members
    pub fn execute<M, A>(self) -> Future<'a, A>
    where
        M: FromValue,
        A: FromSingleValueArray<M>,
    {
        self.geo_commands.send_into(self.cmd)
    }

    /// Also return the longitude and latitude of the matching items.
    pub fn with_coord(self) -> GeoSearchWithOptions<'a, T> {
        GeoSearchWithOptions {
            geo_commands: self.geo_commands,
            cmd: self.cmd.arg("WITHCOORD"),
        }
    }

    /// Also return the distance of the returned items from the specified center point.
    /// The distance is returned in the same unit as specified for the radius or height and width arguments.
    pub fn with_dist(self) -> GeoSearchWithOptions<'a, T> {
        GeoSearchWithOptions {
            geo_commands: self.geo_commands,
            cmd: self.cmd.arg("WITHDIST"),
        }
    }

    /// Also return the raw geohash-encoded sorted set score of the item, in the form of a 52 bit unsigned integer.
    /// This is only useful for low level hacks or debugging and is otherwise of little interest for the general user.
    pub fn with_hash(self) -> GeoSearchWithOptions<'a, T> {
        GeoSearchWithOptions {
            geo_commands: self.geo_commands,
            cmd: self.cmd.arg("WITHHASH"),
        }
    }
}

/// Builder for the [geosearch](crate::GeoCommands::geosearch) command
pub struct GeoSearchWithOptions<'a, T: GeoCommands + ?Sized> {
    geo_commands: &'a T,
    cmd: Command,
}

impl<'a, T: GeoCommands + ?Sized> GeoSearchWithOptions<'a, T> {
    /// # Return
    /// A linear array of members + additional information depending
    /// on which with_xyz options have been selected
    pub fn execute<M, A>(self) -> Future<'a, A>
    where
        M: FromValue,
        A: FromSingleValueArray<GeoSearchResult<M>>,
    {
        self.geo_commands.send_into(self.cmd)
    }

    /// Also return the longitude and latitude of the matching items.
    pub fn with_coord(self) -> GeoSearchWithOptions<'a, T> {
        GeoSearchWithOptions {
            geo_commands: self.geo_commands,
            cmd: self.cmd.arg("WITHCOORD"),
        }
    }

    /// Also return the distance of the returned items from the specified center point.
    /// The distance is returned in the same unit as specified for the radius or height and width arguments.
    pub fn with_dist(self) -> GeoSearchWithOptions<'a, T> {
        GeoSearchWithOptions {
            geo_commands: self.geo_commands,
            cmd: self.cmd.arg("WITHDIST"),
        }
    }

    /// Also return the raw geohash-encoded sorted set score of the item, in the form of a 52 bit unsigned integer.
    /// This is only useful for low level hacks or debugging and is otherwise of little interest for the general user.
    pub fn with_hash(self) -> GeoSearchWithOptions<'a, T> {
        GeoSearchWithOptions {
            geo_commands: self.geo_commands,
            cmd: self.cmd.arg("WITHHASH"),
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
            None => { return Err(Error::Internal("Unexpected geo search result".to_owned())); }
        }

        while let Some(value) = it.next() {
            match value {
                Value::BulkString(BulkString::Binary(_)) => distance = Some(value.into()?),
                Value::Integer(h) => geo_hash = Some(h),
                Value::Array(_) => coordinates = Some(value.into()?),
                _ => return Err(Error::Internal("Unexpected geo search result".to_owned()))
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

/// Builder for the [geosearchstore](crate::GeoCommands::geosearchstore) command
pub struct GeoSearchStore<'a, T: GeoCommands + ?Sized> {
    geo_commands: &'a T,
    cmd: Command,
}

impl<'a, T: GeoCommands + ?Sized> GeoSearchStore<'a, T> {
    /// Matching items are returned unsorted by default. To sort them, use one of the following two options
    /// * [Asc](crate::GeoSearchOrder::Asc): Sort returned items from the nearest to the farthest, relative to the center point.
    /// * [Desc](crate::GeoSearchOrder::Desc): Sort returned items from the farthest to the nearest, relative to the center point.
    pub fn order(self, order: GeoSearchOrder) -> Self {
        Self {
            geo_commands: self.geo_commands,
            cmd: self.cmd.arg(order),
        }
    }

    /// Limit the results to the first `count` matching items,
    /// When the `any` option is used, the command returns as soon as enough matches are found.
    pub fn count(self, count: usize, any: bool) -> Self {
        let mut cmd = self.cmd.arg("COUNT").arg(count);

        if any {
            cmd = cmd.arg("ANY");
        }

        Self {
            geo_commands: self.geo_commands,
            cmd,
        }
    }

    /// When using the STOREDIST option, the command stores the items in a sorted set populated
    /// with their distance from the center of the circle or box, as a floating-point number,
    /// in the same unit specified for that shape.
    pub fn store_dist(self) -> Self {
        Self {
            geo_commands: self.geo_commands,
            cmd: self.cmd.arg("STOREDIST"),
        }
    }

    /// Executes the command
    /// 
    /// # Return
    /// the number of elements in the resulting set.
    pub fn execute(self) -> Future<'a, usize>
    {
        self.geo_commands.send_into(self.cmd)
    }
}
