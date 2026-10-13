"""Read givenName from the existing YunoHost LDAP, never create an account."""
import re
import sys


def first_name(attributes, uid):
    for key in ("givenName", "displayName", "cn"):
        values = attributes.get(key, [])
        if values:
            value = values[0].decode("utf-8").strip()
            if value:
                if len(value) > 256 or any(ord(char) < 32 for char in value):
                    raise ValueError("Invalid directory attribute")
                return value
    return uid


def resolve(uid):
    if not re.fullmatch(r"[a-zA-Z0-9_.-]{1,64}", uid):
        raise ValueError("Invalid uid")
    import ldap  # Provided by YunoHost; no pip installation.
    from ldap.filter import escape_filter_chars

    connection = ldap.initialize("ldap://127.0.0.1:389")
    connection.set_option(ldap.OPT_NETWORK_TIMEOUT, 3)
    connection.set_option(ldap.OPT_TIMEOUT, 3)
    try:
        connection.simple_bind_s("", "")
        rows = connection.search_s(
            "ou=users,dc=yunohost,dc=org", ldap.SCOPE_SUBTREE,
            f"(&(objectClass=posixAccount)(uid={escape_filter_chars(uid)}))",
            ["givenName", "displayName", "cn"],
        )
        rows = [(dn, attrs) for dn, attrs in rows if dn]
        if len(rows) != 1:
            raise ValueError("Directory identity not found or ambiguous")
        return first_name(rows[0][1], uid)
    finally:
        connection.unbind_s()


if __name__ == "__main__":
    try:
        if len(sys.argv) != 2:
            raise ValueError("Expected one uid")
        print(resolve(sys.argv[1]))
    except Exception:
        # No directory attributes or secrets in logs.
        sys.exit(1)
