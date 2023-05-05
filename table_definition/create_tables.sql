CREATE SCHEMA adv_chat;
CREATE TABLE adv_chat.user(
    user_id bigserial primary key,
    user_name varchar(32),
    user_passwd_hash bytea,
    salt text,
    avatar text,
    friends bigint[],
    group_list bigint[],
    created_at timestamp
);

CREATE TABLE adv_chat.group(
    group_id bigserial primary key,
    group_name varchar(64),
    group_host bigint REFERENCES adv_chat.user,
    admin_list bigint[],
    user_list bigint[],
    created_at timestamp
);

CREATE TABLE adv_chat.private_message(
    message_id bigserial primary key,
    message varchar(4096),
    message_from bigint REFERENCES adv_chat.user,
    message_to bigint REFERENCES adv_chat.user,
    created_at timestamp
);
CREATE TABLE adv_chat.group_message(
    group_message_id bigserial primary key,
    message_from bigint REFERENCES adv_chat.user,
    group_id bigint REFERENCES adv_chat.group,
    group_message varchar(4096),
    created_at timestamp
);

ALTER SEQUENCE adv_chat.user_user_id_seq RESTART WITH 100000;
ALTER SEQUENCE adv_chat.group_group_id_seq RESTART WITH 100000;