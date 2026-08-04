-- The first migration, kept small enough to replace.
create table if not exists users (
    id bigserial primary key,
    email text not null unique,
    created_at timestamptz not null default now()
);
