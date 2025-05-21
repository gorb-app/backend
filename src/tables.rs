use diesel::table;

table! {
    users (uuid) {
        uuid -> Uuid,
        username -> VarChar,
        display_name -> Nullable<VarChar>,
        password -> VarChar,
        email -> VarChar,
        email_verified -> Bool,
        is_deleted -> Bool,
        deleted_at -> Int8,
    }
}

table! {
    instance_permissions (uuid) {
        uuid -> Uuid,
        administrator -> Bool,
    }
}

table! {
    refresh_tokens (token) {
        token -> VarChar,
        uuid -> Uuid,
        created_at -> Int8,
        device_name -> VarChar,
    }
}

table! {
    access_tokens (token) {
        token -> VarChar,
        refresh_token -> VarChar,
        uuid -> Uuid,
        created_at -> Int8
    }
}

table! {
    guilds (uuid) {
        uuid -> Uuid,
        owner_uuid -> Uuid,
        name -> VarChar,
        description -> VarChar
    }
}

table! {
    guild_members (uuid) {
        uuid -> Uuid,
        guild_uuid -> Uuid,
        user_uuid -> Uuid,
        nickname -> VarChar,
    }
}

table! {
    roles (uuid, guild_uuid) {
        uuid -> Uuid,
        guild_uuid -> Uuid,
        name -> VarChar,
        color -> Int4,
        position -> Int4,
        permissions -> Int8,
    }
}

table! {
    role_members (role_uuid, member_uuid) {
        role_uuid -> Uuid,
        member_uuid -> Uuid,
    }
}

table! {
    channels (uuid) {
        uuid -> Uuid,
        guild_uuid -> Uuid,
        name -> VarChar,
        description -> VarChar,
    }
}

table! {
    channel_permissions (channel_uuid, role_uuid) {
        channel_uuid -> Uuid,
        role_uuid -> Uuid,
        permissions -> Int8,
    }
}

table! {
    messages (uuid) {
        uuid -> Uuid,
        channel_uuid -> Uuid,
        user_uuid -> Uuid,
        message -> VarChar,
    }
}

table! {
    invites (id) {
        id -> VarChar,
        guild_uuid -> Uuid,
        user_uuid -> Uuid,
    }
}
