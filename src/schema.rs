// @generated automatically by Diesel CLI.

diesel::table! {
    access_tokens (token) {
        #[max_length = 32]
        token -> Varchar,
        #[max_length = 64]
        refresh_token -> Varchar,
        uuid -> Uuid,
        created_at -> Int8,
    }
}

diesel::table! {
    audit_logs (uuid) {
        uuid -> Uuid,
        guild_uuid -> Uuid,
        action_id -> Int2,
        by_uuid -> Uuid,
        channel_uuid -> Nullable<Uuid>,
        user_uuid -> Nullable<Uuid>,
        message_uuid -> Nullable<Uuid>,
        role_uuid -> Nullable<Uuid>,
        #[max_length = 200]
        audit_message -> Nullable<Varchar>,
        #[max_length = 200]
        changed_from -> Nullable<Varchar>,
        #[max_length = 200]
        changed_to -> Nullable<Varchar>,
    }
}

diesel::table! {
    channel_permissions (channel_uuid, role_uuid) {
        channel_uuid -> Uuid,
        role_uuid -> Uuid,
        permissions -> Int8,
    }
}

diesel::table! {
    channels (uuid) {
        uuid -> Uuid,
        guild_uuid -> Uuid,
        #[max_length = 32]
        name -> Varchar,
        #[max_length = 500]
        description -> Nullable<Varchar>,
        is_above -> Nullable<Uuid>,
    }
}

diesel::table! {
    friend_requests (sender, receiver) {
        sender -> Uuid,
        receiver -> Uuid,
        requested_at -> Timestamptz,
    }
}

diesel::table! {
    friends (uuid1, uuid2) {
        uuid1 -> Uuid,
        uuid2 -> Uuid,
        accepted_at -> Timestamptz,
    }
}

diesel::table! {
    guild_bans (user_uuid, guild_uuid) {
        guild_uuid -> Uuid,
        user_uuid -> Uuid,
        #[max_length = 200]
        reason -> Nullable<Varchar>,
        banned_since -> Timestamptz,
    }
}

diesel::table! {
    guild_members (uuid) {
        uuid -> Uuid,
        guild_uuid -> Uuid,
        user_uuid -> Uuid,
        #[max_length = 100]
        nickname -> Nullable<Varchar>,
        is_owner -> Bool,
    }
}

diesel::table! {
    guilds (uuid) {
        uuid -> Uuid,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 300]
        description -> Nullable<Varchar>,
        #[max_length = 8000]
        icon -> Nullable<Varchar>,
    }
}

diesel::table! {
    instance_permissions (uuid) {
        uuid -> Uuid,
        administrator -> Bool,
    }
}

diesel::table! {
    invites (id) {
        #[max_length = 32]
        id -> Varchar,
        guild_uuid -> Uuid,
        user_uuid -> Uuid,
    }
}

diesel::table! {
    messages (uuid) {
        uuid -> Uuid,
        channel_uuid -> Uuid,
        user_uuid -> Uuid,
        #[max_length = 4000]
        message -> Varchar,
        reply_to -> Nullable<Uuid>,
    }
}

diesel::table! {
    refresh_tokens (token) {
        #[max_length = 64]
        token -> Varchar,
        uuid -> Uuid,
        created_at -> Int8,
        #[max_length = 64]
        device_name -> Varchar,
    }
}

diesel::table! {
    role_members (role_uuid, member_uuid) {
        role_uuid -> Uuid,
        member_uuid -> Uuid,
    }
}

diesel::table! {
    roles (uuid) {
        uuid -> Uuid,
        guild_uuid -> Uuid,
        #[max_length = 50]
        name -> Varchar,
        color -> Int4,
        permissions -> Int8,
        is_above -> Nullable<Uuid>,
    }
}

diesel::table! {
    users (uuid) {
        uuid -> Uuid,
        #[max_length = 32]
        username -> Varchar,
        #[max_length = 64]
        display_name -> Nullable<Varchar>,
        #[max_length = 512]
        password -> Varchar,
        #[max_length = 100]
        email -> Varchar,
        email_verified -> Bool,
        is_deleted -> Bool,
        deleted_at -> Nullable<Int8>,
        #[max_length = 8000]
        avatar -> Nullable<Varchar>,
        #[max_length = 32]
        pronouns -> Nullable<Varchar>,
        #[max_length = 200]
        about -> Nullable<Varchar>,
        online_status -> Int2,
    }
}

diesel::joinable!(access_tokens -> refresh_tokens (refresh_token));
diesel::joinable!(access_tokens -> users (uuid));
diesel::joinable!(audit_logs -> channels (channel_uuid));
diesel::joinable!(audit_logs -> guild_members (by_uuid));
diesel::joinable!(audit_logs -> messages (message_uuid));
diesel::joinable!(audit_logs -> roles (role_uuid));
diesel::joinable!(audit_logs -> users (user_uuid));
diesel::joinable!(channel_permissions -> channels (channel_uuid));
diesel::joinable!(channel_permissions -> roles (role_uuid));
diesel::joinable!(channels -> guilds (guild_uuid));
diesel::joinable!(guild_bans -> guilds (guild_uuid));
diesel::joinable!(guild_bans -> users (user_uuid));
diesel::joinable!(guild_members -> guilds (guild_uuid));
diesel::joinable!(guild_members -> users (user_uuid));
diesel::joinable!(instance_permissions -> users (uuid));
diesel::joinable!(invites -> guilds (guild_uuid));
diesel::joinable!(invites -> users (user_uuid));
diesel::joinable!(messages -> channels (channel_uuid));
diesel::joinable!(messages -> users (user_uuid));
diesel::joinable!(refresh_tokens -> users (uuid));
diesel::joinable!(role_members -> guild_members (member_uuid));
diesel::joinable!(role_members -> roles (role_uuid));
diesel::joinable!(roles -> guilds (guild_uuid));

diesel::allow_tables_to_appear_in_same_query!(
    access_tokens,
    audit_logs,
    channel_permissions,
    channels,
    friend_requests,
    friends,
    guild_bans,
    guild_members,
    guilds,
    instance_permissions,
    invites,
    messages,
    refresh_tokens,
    role_members,
    roles,
    users,
);
