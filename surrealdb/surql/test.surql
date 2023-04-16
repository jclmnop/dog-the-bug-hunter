-- Create new report
BEGIN TRANSACTION;
LET $report_id = fn::report_id($auth.id, $timestamp, $target);
CREATE $report_id SET user = type::thing("user", $token.ID), timestamp = <datetime> $timestamp, subdomains = [];
--
-- TODO: index on subdomain and port?
-- Repeat this for each subdomain
LET $report_id = fn::report_id($auth.id, $timestamp, $target);
CREATE subdomain SET subdomain = $subdomain.subdomain, report = $report_id, open_ports = [];
LET $subdomain_id = (SELECT id FROM subdomain WHERE subdomain = $subdomain.subdomain and report = $report_id).id;
UPDATE $report_id MERGE {
    subdomains: array::append($report_id.subdomains, $subdomain_id)
};
--

    -- Repeat this for each port
    LET $port =
        CREATE port SET subdomain = $subdomain_id, findings = [], is_open = true RETURN id;
    UPDATE $subdomain_id MERGE {
        ports: array::append($subdomain_id.ports, $port)
    };


-- OR
LET $subdomain_id =
    CREATE subdomain SET subdomain = $subdomain, report = $report_id RETURN id;
LET $port =
    CREATE port SET port = $port, subdomain = $subdomain_id, findings = [], is_open = true RETURN id;
LET $ports = (SELECT id FROM port WHERE subdomain = $subdomain_id);
RELATE $subdomain_id->ports->$ports;

LET $subdomains = (SELECT id FROM subdomain WHERE report = $report_id);
RELATE $report_id->subdomains->$subdomains;

