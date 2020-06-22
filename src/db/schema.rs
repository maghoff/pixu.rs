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
    pixur_authorizations (pixur_id, sub) {
        pixur_id -> Integer,
        sub -> Text,
    }
}

table! {
    pixurs (id) {
        id -> Integer,
        average_color -> Integer,
        thumbs_id -> Integer,
        created -> Timestamp,
        image_aspect_ratio -> Float,
        crop_left -> Float,
        crop_right -> Float,
        crop_top -> Float,
        crop_bottom -> Float,
    }
}

table! {
    pixurs_series (id, order) {
        id -> Integer,
        order -> Integer,
        pixurs_id -> Integer,
    }
}

table! {
    thumbs (id) {
        id -> Integer,
        media_type -> Text,
        data -> Binary,
    }
}

table! {
    uploaders (sub) {
        sub -> Text,
    }
}

joinable!(images_meta -> images (id));
joinable!(images_meta -> pixurs (pixurs_id));
joinable!(pixur_authorizations -> pixurs (pixur_id));
joinable!(pixurs -> thumbs (thumbs_id));
joinable!(pixurs_series -> pixurs (pixurs_id));

allow_tables_to_appear_in_same_query!(
    images,
    images_meta,
    pixur_authorizations,
    pixurs,
    pixurs_series,
    thumbs,
    uploaders,
);
