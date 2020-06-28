DROP TABLE pixur_series;

CREATE TABLE pixur_authorizations (
    pixur_id INTEGER NOT NULL,
    sub TEXT NOT NULL,

    PRIMARY KEY (pixur_id, sub),
    FOREIGN KEY (pixur_id) REFERENCES pixurs(id)
);

INSERT INTO pixur_authorizations (pixur_id, sub)
    SELECT pixur_series_id, sub FROM pixur_series_authorizations JOIN pixurs ON pixur_series_id = pixurs.id;

DROP TABLE pixur_series_authorizations;
