#!/usr/bin/env bash

diesel migration run --database-url=diesel.db
diesel print-schema --database-url=diesel.db > src/db/schema.rs
rm diesel.db
