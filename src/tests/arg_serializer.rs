use crate::{
    commands::{
        FtAggregateOptions, FtAttribute, FtFieldSchema, FtFieldType, FtGroupBy,
        FtHnswVectorFieldAttributes, FtReducer, FtSearchHighlightOptions, FtSearchOptions,
        FtSortBy, FtSortByProperty, FtSpellCheckOptions, FtTermType, FtVectorDistanceMetric,
        FtVectorFieldAlgorithm, FtVectorType, SetExpiration, SortOptions, SortOrder,
    },
    resp::{ArgCounter, ArgSerializer, BulkString, RefBulkString, cmd},
};
use bytes::BytesMut;
use serde::Serialize;

#[test]
pub(super) fn byte_slice() {
    let mut buffer = BytesMut::new();
    let mut serializer = ArgSerializer::from_buffer(&mut buffer);
    RefBulkString::from(b"foo")
        .serialize(&mut serializer)
        .unwrap();
    RefBulkString::from(b"bar")
        .serialize(&mut serializer)
        .unwrap();
    assert_eq!(
        "$3\r\nfoo\r\n$3\r\nbar\r\n",
        str::from_utf8(buffer.freeze().as_ref()).unwrap()
    );
}

#[test]
pub(super) fn bute_vec() {
    let mut buffer = BytesMut::new();
    let mut serializer = ArgSerializer::from_buffer(&mut buffer);
    BulkString::from(b"foo".to_vec())
        .serialize(&mut serializer)
        .unwrap();
    BulkString::from(b"bar".to_vec())
        .serialize(&mut serializer)
        .unwrap();
    assert_eq!(
        "$3\r\nfoo\r\n$3\r\nbar\r\n",
        str::from_utf8(buffer.freeze().as_ref()).unwrap()
    );
}

/// Every clause that declares an argument count to the server derives that
/// count from an `ArgCounter` dry run, which is only correct as long as
/// `ArgCounter` and `ArgSerializer` walk a value identically. They are separate
/// implementations of the same traversal and have drifted apart before, on
/// empty-named struct fields, so the agreement is asserted here on the shapes
/// where it is not obvious: renamed and skipped fields, flags, nested options,
/// enum variants and collections of structs.
#[test]
pub(super) fn arg_counter_agrees_with_arg_serializer() {
    fn assert_agree<T: Serialize>(label: &str, value: T) {
        let mut counter = ArgCounter::default();
        value.serialize(&mut counter).unwrap();

        let written = cmd("CMD").arg(value).args_layout.len();

        assert_eq!(
            counter.count, written,
            "{label}: the dry run counted {} arguments, {written} were written",
            counter.count
        );
    }

    assert_agree("empty", FtSearchOptions::default());
    assert_agree("flag", FtSearchOptions::default().nocontent());
    assert_agree(
        "renamed attribute",
        FtSearchOptions::default()._return(FtAttribute::new("a").r#as("b")),
    );
    assert_agree(
        "counted pairs",
        FtSearchOptions::default().param("n", "v").param("n2", "v2"),
    );
    assert_agree(
        "nested options",
        FtSearchOptions::default()
            .highlight(FtSearchHighlightOptions::default().fields("a").fields("b"))
            .sortby("a", SortOrder::Desc, true)
            .limit(0, 10)
            .dialect(2),
    );
    assert_agree(
        "aggregate pipeline",
        FtAggregateOptions::default()
            .load(FtAttribute::new("a").r#as("b"))
            .groupby(
                FtGroupBy::default()
                    .property("@a")
                    .reduce(FtReducer::count().as_name("cnt")),
            )
            .sortby(FtSortBy::default().property(FtSortByProperty::new("@cnt").desc()))
            .param("n", "v"),
    );
    assert_agree(
        "vector field",
        FtFieldSchema::identifier("v").field_type(FtFieldType::Vector(Some(
            FtVectorFieldAlgorithm::HNSW(
                FtHnswVectorFieldAttributes::new(
                    FtVectorType::Float32,
                    4,
                    FtVectorDistanceMetric::Cosine,
                )
                .m(16)
                .epsilon(0.01),
            ),
        ))),
    );
    assert_agree(
        "spellcheck terms",
        FtSpellCheckOptions::default()
            .distance(2)
            .terms(FtTermType::Include, "dict"),
    );
    assert_agree("sort", SortOptions::default().by("weight_*").limit(0, 10));
    assert_agree("expiration", SetExpiration::Ex(60));
}
