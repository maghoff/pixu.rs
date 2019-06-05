CREATE TABLE pixur_authorizations (
    pixur_id INTEGER NOT NULL,
    sub TEXT NOT NULL,

    PRIMARY KEY (pixur_id, sub),
    FOREIGN KEY (pixur_id) REFERENCES pixurs(id)
);
