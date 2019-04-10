table! {
    images (id) {
        id -> Integer,
        media_type -> Text,
        data -> Binary,
    }
}

table! {
    images_meta (id) {
        id -> Integer,
        width -> Integer,
        height -> Integer,
        pixurs_id -> Integer,
    }
}

table! {
    pixurs (id) {
        id -> Integer,
        average_color -> Integer,
        thumbs_id -> Integer,
    }
}

table! {
    thumbs (id) {
        id -> Integer,
        media_type -> Text,
        data -> Binary,
    }
}

joinable!(images_meta -> images (id));
joinable!(images_meta -> pixurs (pixurs_id));
joinable!(pixurs -> thumbs (thumbs_id));

allow_tables_to_appear_in_same_query!(
    images,
    images_meta,
    pixurs,
    thumbs,
);
