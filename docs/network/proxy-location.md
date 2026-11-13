# Proxy location consistency

A site can compare `Intl.DateTimeFormat().resolvedOptions().timeZone` and `navigator.languages` with
the location of the IP address it sees. Veneer never looks up IP addresses: you supply what your proxy
provider reports, and `check_location` compares it with the profile.

```bash
veneer net location --profile profile.json --country TR --timezone Europe/Istanbul
curl -s -X POST http://127.0.0.1:8787/v1/network/location \
  -H 'content-type: application/json' -d @body.json   # {"profile": {...}, "country": "TR", "timezone": "Europe/Istanbul"}
```

| Rule | Severity | Fires when |
| --- | --- | --- |
| `location.timezone` | error | the given timezone differs from `profile.timezone` |
| `location.country` | error | the profile's timezone is not one the locale dataset uses in that country |
| `location.locale` | warning | the locale's region is another country (common for travellers and expatriates) |

Countries missing from the locale dataset are reported in `notes`, and only the locale region is
compared for them. To get a matching profile in the first place, generate it with `--country` and
`--timezone` set to the proxy's values.
