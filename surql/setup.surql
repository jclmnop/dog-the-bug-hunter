-- Setup
-- -- Functions
DEFINE FUNCTION fn::report_id($token_id: string, $timestamp: int, $target: string) {
    RETURN type::thing(
        "report",
        crypto::md5(string::concat($token_id, $timestamp, $target))
);}

DEFINE FUNCTION fn::report_exists($report_id: record()) {
    RETURN $report_id IN (SELECT id FROM report).id;
};
-- -- Scopes
DEFINE SCOPE user_scope
SESSION 14d
SIGNUP (
    CREATE type::thing("user", string::lowercase(string::trim($username)))
    SET pass = crypto::argon2::generate($password)
)
SIGNIN (
    SELECT * FROM type::thing("user", string::lowercase(string::trim($username)))
    WHERE crypto::argon2::compare(pass, $password)
);
-- -- Tables
-- -- -- Report Table
DEFINE TABLE report SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
        WHERE user = $token.id AND $scope = "user_scope";
DEFINE FIELD user ON TABLE report TYPE record (user)
    ASSERT $value != NONE;
DEFINE FIELD target ON TABLE report TYPE string
    ASSERT $value != NONE AND is::url($value);
DEFINE FIELD timestamp ON TABLE report TYPE datetime
    ASSERT $value != NONE;
DEFINE FIELD subdomains ON TABLE report TYPE array
    VALUE $value OR [];
DEFINE FIELD subdomains.* ON TABLE report TYPE record (subdomain);

DEFINE TABLE subdomain SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
        WHERE report.user = $token.id AND $scope = "user_scope";
DEFINE FIELD report ON TABLE subdomain TYPE record (report)
    ASSERT $value != NONE AND fn::report_exists($value);
DEFINE FIELD subdomain_url ON TABLE subdomain TYPE string
    ASSERT $value != NONE AND is::url($value);
DEFINE FIELD ports ON TABLE subdomain TYPE array
    VALUE $value OR [];
DEFINE FIELD ports.* ON TABLE subdomain TYPE record (port);

DEFINE TABLE port SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
        WHERE subdomain.report.user = $token.id AND $scope = "user_scope";
DEFINE FIELD subdomain ON TABLE port TYPE record (subdomain)
    ASSERT $value != NONE;
DEFINE FIELD port ON TABLE port TYPE int
    ASSERT $value != NONE;
DEFINE FIELD findings ON TABLE port TYPE array
    VALUE $value OR [];
DEFINE FIELD is_open ON TABLE port TYPE boolean
    VALUE $value OR true;
DEFINE FIELD findings.* ON TABLE port TYPE object;

-- TODO: define events