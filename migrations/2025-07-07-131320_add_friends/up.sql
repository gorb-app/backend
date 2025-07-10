-- Your SQL goes here
CREATE TABLE friends (
    uuid1 UUID REFERENCES users(uuid),
    uuid2 UUID REFERENCES users(uuid),
    accepted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (uuid1, uuid2),
    CHECK (uuid1 < uuid2)
);

CREATE TABLE friend_requests (
    sender UUID REFERENCES users(uuid),
    receiver UUID REFERENCES users(uuid),
    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (sender, receiver),
    CHECK (sender <> receiver)
);

-- Create a function to check for existing friendships
CREATE FUNCTION check_friend_request()
RETURNS TRIGGER AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM friends 
        WHERE (uuid1, uuid2) = (LEAST(NEW.sender, NEW.receiver), GREATEST(NEW.sender, NEW.receiver))
    ) THEN
        RAISE EXCEPTION 'Users are already friends';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create the trigger
CREATE TRIGGER prevent_friend_request_conflict
BEFORE INSERT OR UPDATE ON friend_requests
FOR EACH ROW EXECUTE FUNCTION check_friend_request();
