CREATE TABLE pixur_series (
    id INTEGER NOT NULL,
    'order' INTEGER NOT NULL,
    pixurs_id INTEGER NOT NULL,

    PRIMARY KEY (id, 'order'),
    FOREIGN KEY (pixurs_id) REFERENCES pixurs(id)
);

INSERT INTO pixur_series (id, 'order', pixurs_id)
    SELECT id, 0, id FROM pixurs;

CREATE TABLE pixur_series_authorizations (
    pixur_series_id INTEGER NOT NULL,
    sub TEXT NOT NULL,

    PRIMARY KEY (pixur_series_id, sub)
);

INSERT INTO pixur_series_authorizations (pixur_series_id, sub)
    SELECT pixur_id, sub FROM pixur_authorizations;

DROP TABLE pixur_authorizations;
