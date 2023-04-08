create table shared_folders
(
    id   blob,
    path text,

    primary key (id),
    unique (path)
)