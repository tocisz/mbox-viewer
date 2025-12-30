
import email
import email.policy
import email.utils
from datetime import datetime

def parse_date(date_str):
    if not date_str:
        return datetime.now()
    try:
        # parsedate_to_datetime handles many formats
        return email.utils.parsedate_to_datetime(date_str)
    except Exception as e:
        print(f"Error parsing '{date_str}': {e}")
        return datetime.now()

def test_date_parsing():
    # Test case 1: Standard RFC date
    raw = b"Date: Wed, 30 Dec 2025 20:20:00 +0000\n\n"
    msg = email.message_from_bytes(raw, policy=email.policy.default)
    date_val = msg.get("Date")
    print(f"Type of Date header: {type(date_val)}")
    print(f"Value of Date header: {date_val}")
    parsed = parse_date(date_val)
    print(f"Parsed result: {parsed} (type: {type(parsed)})")

    # Test case 2: Non-standard date (like the error)
    # Note: with default policy, if the header is invalid, it might still return a UnstructuredHeader or similar
    raw_bad = b"Date: 11/04/2021\n\n"
    msg_bad = email.message_from_bytes(raw_bad, policy=email.policy.default)
    date_val_bad = msg_bad.get("Date")
    print(f"Type of Bad Date header: {type(date_val_bad)}")
    print(f"Value of Bad Date header: {date_val_bad}")
    parsed_bad = parse_date(date_val_bad)
    print(f"Parsed Bad result: {parsed_bad} (type: {type(parsed_bad)})")
    
    # Test case 3: passing string directly
    print(f"Direct text parse: {parse_date('11/04/2021')}")

if __name__ == "__main__":
    test_date_parsing()
