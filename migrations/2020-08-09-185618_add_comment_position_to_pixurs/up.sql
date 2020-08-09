ALTER TABLE pixurs
    ADD comment_position TEXT NOT NULL
        DEFAULT "bottom"
        CHECK (comment_position IN ("top", "center", "bottom"));
