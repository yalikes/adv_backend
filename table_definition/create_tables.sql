CREATE TABLE private_message(
    message_id bigserial primary key,
    message varchar(4096),
    message_from REFERENCES user,
    message_to REFERENCES user,
    created_at timestamp
);
CREATE TABLE group_message(
    group_message_id bigserial primary key,
    group_id REFERENCES group
    group_message varchar(4096),
    message_from REFERENCES user,
    group_admin REFERENCES user,
    admin_list bigint[],
);

CREATE TABLE group(

);

CREATE TABLE user(

);
