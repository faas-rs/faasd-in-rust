-- Your SQL goes here
CREATE TABLE users (
    uid UUID PRIMARY KEY DEFAULT gen_random_uuid(), -- 用户唯一 ID（32位随机）
    username VARCHAR NOT NULL UNIQUE,
    password_hash VARCHAR NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
