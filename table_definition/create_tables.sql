CREATE TABLE private_message(
    message_id bigserial primary key,
    message varchar(4096),
    message_from REFERENCES user,
    message_to REFERENCES user,
    created_at timestamp
);
CREATE TABLE group_message(
    group_message_id bigserial primary key,
    message_from REFERENCES user,
    group_id REFERENCES group
    group_message varchar(4096),
    created_at timestamp
);

CREATE TABLE group(
    group_id serial primary key,
    group_name varchar(64),
    group_host REFERENCES user,
    admin_list bigint[],
    user_list bigint[],
    created_at timestamp
);

CREATE TABLE user(
    user_id serial primary key,
    user_name varchar(32),
    user_passwd_hash bytea,
    salt text,
    created_at timestamp
);
