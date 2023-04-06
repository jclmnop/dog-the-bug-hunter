-- Create users table
CREATE TABLE users (
    user_id SERIAL PRIMARY KEY,
    hashed_password VARCHAR(255) NOT NULL
);

-- Create targets table
CREATE TABLE targets (
    target_id SERIAL PRIMARY KEY,
    target_name VARCHAR(255) NOT NULL
);

-- Create subdomains table
CREATE TABLE subdomains (
    subdomain_id SERIAL PRIMARY KEY,
    subdomain_name VARCHAR(255) NOT NULL,
    target_id INTEGER NOT NULL,
    FOREIGN KEY (target_id) REFERENCES targets(target_id)
);

-- Create user_targets table
CREATE TABLE user_targets (
    user_target_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    target_id INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(user_id),
    FOREIGN KEY (target_id) REFERENCES targets(target_id)
);

-- -- Create user_target_subdomains table
-- CREATE TABLE user_target_subdomains (
--     user_target_subdomain_id SERIAL PRIMARY KEY,
--     user_target_id INTEGER NOT NULL,
--     subdomain_id INTEGER NOT NULL,
--     FOREIGN KEY (user_target_id) REFERENCES user_targets(user_target_id),
--     FOREIGN KEY (subdomain_id) REFERENCES subdomains(subdomain_id)
-- );

-- Create ports table
CREATE TABLE ports (
    port_id SERIAL PRIMARY KEY,
    port_number INTEGER NOT NULL,
    is_open BOOLEAN NOT NULL,
    user_target_id INTEGER NOT NULL,
    subdomain_id INTEGER NOT NULL,
    FOREIGN KEY (user_target_id) REFERENCES user_target_subdomains(user_target_id),
    FOREIGN KEY (subdomain_id) REFERENCES subdomains(subdomain_id)
);

-- Create findings table
CREATE TABLE findings (
    finding_id SERIAL PRIMARY KEY,
    url VARCHAR(255) NOT NULL,
    finding_type VARCHAR(255) NOT NULL,
    port_id INTEGER NOT NULL,
    FOREIGN KEY (port_id) REFERENCES ports(port_id)
);

-- Create reports view
CREATE VIEW reports AS
SELECT
    u.user_id,
--     t.target_id,
    t.target_name,
--     s.subdomain_id,
    s.subdomain_name,
--     ut.user_target_id,
    ut.timestamp,
--     uts.user_target_subdomain_id,
--     p.port_id,
    p.port_number,
    p.is_open,
--     f.finding_id,
    f.url AS finding_url,
    f.finding_type
FROM
    users u
JOIN
    user_targets ut ON u.user_id = ut.user_id
JOIN
    targets t ON ut.target_id = t.target_id
-- JOIN
--     user_target_subdomains uts ON ut.user_target_id = uts.user_target_id
JOIN
    subdomains s ON s.target_id = t.target_id
JOIN
    ports p ON ut.user_target_id = p.user_target_id AND s.subdomain_id = p.subdomain_id
JOIN
    findings f ON p.port_id = f.port_id;
