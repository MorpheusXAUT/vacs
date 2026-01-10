# Stations configuration reference

> [!IMPORTANT]
> This functionality was implemented as a stop-gap measure to allow people to customize their vacs client a little bit while we work on our "proper" layout featuring a GEO/tabbed main page. It is provided as-is and will not be extended further as the major rework currently in progress will remove most, if not all of the current functionality again.  
> Some limitations apply, such as aliasing only considering frequencies for now, meaning two stations with an identical frequency cannot be aliased independently from each other.

This reference explains how to configure station filtering, prioritization, and display using the `StationsConfig` settings, read from the (optional) `stations.toml` config file.

For general information on the configuration file format, file locations, and recommended editors, please refer to the [main configuration reference](README.md).

## Overview

The `stations` configuration allows you to customize how stations are displayed and filtered in the client. It consists of the following sections:

- **[Profiles](#profiles)** - Define (multiple) filtering configurations that you can switch between in the UI

> [!TIP]  
> You can load an additional stations configuration file (e.g., from a shared profile or sector file package) using the `extra_stations_config` setting in your `client.toml` file.  
> See the [client configuration reference](client.md#extra-stations-config) for more details.

> [!IMPORTANT]  
> These settings are purely client-side and do not prevent a different user from calling you, even if your filters do not match their callsign and you thus cannot see them.  
> If you are receiving a call from a station you cannot currently see, they will still have their respective callsign shown in the call display, however, you **will not** be able to call them back.

## Configuration structure

```toml
# Default configuration
[stations]
ignored = []

# Profiles for filtering and prioritizing stations
[stations.profiles.Default]
include = []
exclude = []
priority = ["*_FMP", "*_CTR", "*_APP", "*_TWR", "*_GND"]
frequencies = "ShowAll"
grouping = "None"

[stations.profiles.Default.aliases]
# Aliases for stations, mapping frequencies to callsigns, e.g.:
# "124.400" = "FIC_CTR"
```

---

## Profiles

Profiles allow you to define multiple filtering configurations and switch between them in the UI. Each profile controls which stations are shown and how they're ordered using three main settings:

- **`include`** – Allowlist patterns for stations to show
- **`exclude`** – Blocklist patterns for stations to hide
- **`priority`** – Ordered patterns that determine display order

### Profile structure

```toml
# Define multiple profiles under [stations.profiles.]
[stations.profiles.Default]
include = []
exclude = []
priority = ["*_FMP", "*_CTR", "*_APP", "*_TWR", "*_GND"]
frequencies = "ShowAll"
grouping = "None"

[stations.profiles.CentersOnly]
include = ["*_CTR"]
exclude = []
priority = ["LOVV_CTR", "EDMM_CTR"]
frequencies = "HideAliased"
grouping = "FirAndIcao"

[stations.profiles.LOVVOnly]
include = ["LO*"]
exclude = ["LON*"]
priority = ["*_FMP", "*_CTR", "*_APP", "*_TWR", "*_GND"]
frequencies = "HideAll"
grouping = "Icao"
```

### Profile names

Profile names (the part after `stations.profiles.`) can contain:

- Letters (a-z, A-Z)
- Numbers (0-9)
- Underscores (`_`)
- Hyphens (`-`)

These names will be displayed in the UI for profile selection.

### Profile settings

Each profile supports the following settings:

#### `include`: selecting which stations to show

**Type:** Array of strings ([glob patterns](#glob-pattern-matching))  
**Default:** `[]` (empty array)  
**Optional:** Yes

Controls which stations are eligible to be displayed.

- **If empty** (default): All stations are eligible, subject to `exclude` rules
- **If not empty**: Only stations matching at least one pattern are eligible, all other connected clients are hidden.

**Examples:**

```toml
[stations.profiles.local_area]
# Show only Austrian and Munich stations
include = ["LO*", "EDDM_*", "EDMM_*"]

[stations.profiles.app_ctr]
# Show only approach and center controllers
include = ["*_APP", "*_CTR"]

[stations.profiles.vienna]
# Show everything from Vienna airport
include = ["LOWW_*"]
```

---

#### `exclude`: hiding specific stations

**Type:** Array of strings ([glob patterns](#glob-pattern-matching))  
**Default:** `[]` (empty array)  
**Optional:** Yes

Excludes specific stations from being displayed. Exclude rules always take precedence over `include` rules, allowing you to e.g., include a whole FIR, but exclude all of their ground stations.

**Examples:**

```toml
[stations.profiles.hide_all_ground]
# Hide all ground, tower, and delivery positions
exclude = ["*_TWR", "*_GND", "*_DEL"]

[stations.profiles.hide_airports]
# Hide specific airports
exclude = ["LOWL_*", "LOWG_*"]

[stations.profiles.hide_fmp]
# Hide flow management positions
exclude = ["*_FPM"]
```

---

#### `priority`: ordering stations

**Type:** Ordered array of strings ([glob patterns](#glob-pattern-matching))  
**Default:** `["*_FMP", "*_CTR", "*_APP", "*_TWR", "*_GND"]`  
**Optional:** Yes

Determines the display order of stations. The first matching pattern assigns the station's priority bucket – earlier patterns = higher priority.

Stations are grouped by their priority bucket and then sorted within each bucket. Stations that don't match any priority pattern appear last. After grouping, stations are sorted in alphabetical order (ascending) within their respective buckets.

**Default behavior:**

The default priority list orders stations by controller type:

1. Flow Management Positions (`*_FMP`)
2. Center controllers (`*_CTR`)
3. Approach controllers (`*_APP`)
4. Tower controllers (`*_TWR`)
5. Ground controllers (`*_GND`)

Stations not matched by the `priority` setting are grouped by their station type (alphabetical order, ascending), followed by the remaining stations _without_ a valid type (should only appear on `dev` server).

> [!TIP]  
> If you're trying to completely disable the default behavior, set `priority` to an empty array (`[]`).  
> If you omit the value from your config file, the default will be used.

**Examples:**

```toml
[stations.profiles.local_area]
# Prioritize your local area
priority = [
  "LOVV_*",           # Austrian center first
  "LOWW_*_APP",       # Vienna approach
  "LOWW_*_TWR",       # Vienna tower
  "LOWW_*",           # Other Vienna positions
  "*_CTR",            # Other centers
  "*_APP"             # Other approaches
]

[stations.profiles.centers_first]
# Simple setup: centers, then everything else (grouped by type, alphabetically, ascending)
priority = ["*_CTR"]
```

---

#### `aliases`: customizing station display names

**Type:** Table/Dictionary mapping frequencies to display names  
**Default:** `{}` (empty)  
**Optional:** Yes

Allows you to override the display name of stations based on their frequency. This is useful when VATSIM callsigns don't match your preferred display names, or when you want to provide stable naming regardless of which controller is online.

> [!IMPORTANT]  
> Display names must follow the same underscore-separated format as VATSIM callsigns (e.g., `Station_Name_TYPE`) to ensure proper filtering, sorting, and display.  
> Underscores will be replaced with spaces in the UI.  
> The last part after the final underscore is used as the station type.

**When to use aliases:**

- Customizing station names when VATSIM callsigns don't match sector naming conventions
- Using local language or abbreviations (e.g., "VN_APP" instead of "LOWW_N_APP")
- Providing consistent naming when controllers use personalized or relief callsigns

**Frequency matching:**

- Matching is **exact** (no wildcard support)
- Frequencies must match the format received from VATSIM (`1xx.xxx`, decimal point)
- If a station's frequency matches a key in the aliases table, the mapped display name replaces the original

**Examples:**

```toml
[stations.profiles.SectorNames]
# Basic aliases for sectors
[stations.profiles.SectorNames.aliases]
"124.400" = "FIC_CTR"
"134.675" = "VB_APP"

[stations.profiles.Callsigns]
# Full station callsign
[stations.profiles.Callsigns.aliases]
"134.675" = "Wien_Radar_APP" # Will show as "Wien Radar" in the UI

[stations.profiles.Standardized]
# Standardize sector names regardless of who's online/relief callsign use
[stations.profiles.Standardized.aliases]
"132.600" = "LOVV_CTR"
```

> [!IMPORTANT]  
> **How aliases interact with other settings:**
>
> - **Filtering (`include`/`exclude`)**: Operates on the **original** VATSIM callsign, not the aliased one
> - **Priority matching**: Uses the **aliased** VATSIM callsign for pattern matching
> - **Sorting**: Alphabetical sorting uses the **aliased** display name
> - **Display**: Shows the **aliased** name in the UI. The original callsign is displayed while hovering for DA key
>
> This means you will need to make sure your filter patterns match your original callsigns instead of the aliased ones (e.g., `LOWW_*_APP` instead of `Wien_Radar_*_APP`).

**Example with filtering:**

```toml
[stations.profiles.FIC]
include = ["LO*"]
exclude = ["LON*"]
priority = ["*_CTR", "*_APP", "*_TWR", "*_GND"]

[stations.profiles.FIC.aliases]
"124.400" = "FIC_CTR"
```

---

#### `frequencies`: controlling frequency display

**Type:** String (Enum)  
**Default:** `"ShowAll"`  
**Optional:** Yes

Controls how frequencies are displayed on the DA keys in the UI.

**Valid values:**

- `"ShowAll"` (default): Always show frequencies for all stations.
- `"HideAliased"`: Hide frequencies only for stations that have an alias defined in the [`aliases`](#aliases-customizing-station-display-names) section.
- `"HideAll"`: Never show frequencies for any station.

**Example:**

```toml
[stations.profiles.Clean]
# Hide frequencies for aliased stations to reduce clutter
frequencies = "HideAliased"

[stations.profiles.Compact]
# Hide all frequencies to save space
frequencies = "HideAll"
```

---

#### `grouping`: controlling station grouping

**Type:** String (Enum)  
**Default:** `"None"`  
**Optional:** Yes

Controls how DA keys are grouped.

**Valid values:**

- `"None"` (default): Don't group.
- `"Fir"`: Group by the first two letters (FIR) of the display name.
- `"FirAndIcao"`: First, group by the first two letters (FIR), then by the first four letters (ICAO code) of the display name.
- `"Icao"`: Group by the first four letters (ICAO code) of the display name.

**Example:**

```toml
[stations.profiles.GroupByFir]
# Group stations by FIR (e.g. LO, ED, ...)
grouping = "Fir"
```

##### Grouping Logic Examples

The following examples assume these stations are online:
`LOVV_CTR`, `LOWW_APP`, `LOWW_TWR`, `LOWW_GND`, `EDMM_CTR`, `EDDM_TWR`

**`grouping = "Fir"` or `"Icao"` (Single Layer)**

These modes create a simple one-level grouping structure.

**`Fir`**: Groups by the first two letters.

- **LO**
    - LOVV_CTR
    - LOWW_APP
    - LOWW_TWR
    - LOWW_GND
- **ED**
    - EDMM_CTR
    - EDDM_TWR

**`Icao`**: Groups by the first four letters.

- **LOVV**
    - LOVV_CTR
- **LOWW**
    - LOWW_APP
    - LOWW_TWR
    - LOWW_GND
- **EDMM**
    - EDMM_CTR
- **EDDM**
    - EDDM_TWR

**`grouping = "FirAndIcao"` (Two Layers)**

This mode creates a hierarchical structure, first grouping by FIR, then by ICAO code within that FIR.

- **LO**
    - **LOVV**
        - LOVV_CTR
    - **LOWW**
        - LOWW_APP
        - LOWW_TWR
        - LOWW_GND
- **ED**
    - **EDMM**
        - EDMM_CTR
    - **EDDM**
        - EDDM_TWR

---

### Glob pattern matching

All patterns use glob-like syntax, which provides flexible matching with wildcards:

#### Wildcards

- **`*`** – Matches zero or more characters
- **`?`** – Matches exactly one character

#### Matching rules

- Matching is **case-insensitive** (`loww` matches `LOWW`)
- Patterns must match the **entire callsign** (anchored at start and end)
    - If you want to match a substring in the middle, surround it with wildcards (e.g., `*WW*`)
- The pattern is converted to a regular expression where:
    - `*` becomes `.*` (any characters)
    - `?` becomes `.` (single character)
    - Other regex special characters are escaped

#### Pattern examples

| Pattern      | Matches                                | Doesn't Match             |
| ------------ | -------------------------------------- | ------------------------- |
| `LOWW_*`     | `LOWW_APP`, `LOWW_TWR`, `LOWW_1_TWR`   | `LOWWAPP`, `LOWI_APP`     |
| `*_APP`      | `LOWW_APP`, `EDDM_APP`, `LOVV_S_APP`   | `LOWW_TWR`, `APP`         |
| `LO*`        | `LOWW_APP`, `LOVV_CTR`, `LO123`        | `EDDM_APP`, `XLO`         |
| `LOWW*_APP`  | `LOWW_APP`, `LOWW_M_APP`, `LOWW_1_APP` | `LOWWAPP`, `LOWI_APP`     |
| `LOWW_?_TWR` | `LOWW_1_TWR`, `LOWW_2_TWR`             | `LOWW_TWR`, `LOWW_12_TWR` |
| `*`          | Everything                             | Nothing                   |
| `LOWW_APP`   | `LOWW_APP` (exact match)               | `LOWW_1_APP`              |

#### Common patterns

```toml
# All stations from a country prefix
include = ["LO*"]        # Austria (LOWW, LOWI, LOVV, etc.)
include = ["ED*"]        # Germany
include = ["LH*"]        # Hungary

# All positions at an airport
include = ["LOWW_*"]     # Vienna
include = ["EDDM_*"]     # Munich

# Specific position types everywhere
include = ["*_CTR"]      # All centers
include = ["*_APP"]      # All approaches
include = ["*_TWR"]      # All towers
include = ["*_GND"]      # All ground
include = ["*_DEL"]      # All delivery

# Numbered positions
include = ["LOWW_?_APP"] # LOWW_1_APP, LOWW_2_APP (single digit)
include = ["LOWW_*_APP"] # LOWW_1_APP, LOWW_12_APP (any number)

# Combined patterns
include = ["LOWW_*_TWR"] # All Vienna towers (but not LOWW_TWR)
include = ["ED*_CTR"]    # All German centers
```

---

### How filtering works

Stations are processed in this order:

1. **Include check**: If `include` is not empty, station must match at least one pattern
2. **Exclude check**: Station must not match any `exclude` pattern
3. **Priority assignment**: First matching `priority` pattern determines display order
4. **Display**: Stations are shown grouped and sorted by priority

#### Example walkthrough

Given this configuration:

```toml
[stations.profiles.example]
include = ["LO*", "EDMM_*", "EDDM_*"]
exclude = ["*_GND", "*_DEL"]
priority = ["LOVV*", "*_CTR", "LO*_APP", "*_APP", "*_TWR"]
```

Station processing:

| Callsign       | Include Match? | Exclude Match? | Priority       | Result                  |
| -------------- | -------------- | -------------- | -------------- | ----------------------- |
| `LOVV_CTR`     | ✓ (`LO*`)      | ✗              | 1 (`LOVV*`)    | **Shown, rank 1**       |
| `LOWW_APP`     | ✓ (`LO*`)      | ✗              | 3 (`*_APP`)    | **Shown, rank 3**       |
| `LOWW_GND`     | ✓ (`LO*`)      | ✓ (`*_GND`)    | –              | Hidden                  |
| `EDMM_ALB_CTR` | ✓ (`*_CTR`)    | ✗              | 2 (`*_CTR`)    | **Shown, rank 2**       |
| `EDDM_TWR`     | ✓ (`EDDM_*`)   | ✗              | 4 (`*_TWR`)    | **Shown, rank 4**       |
| `EDDF_APP`     | ✗              | ✗              | –              | Hidden (not in include) |
| `LON_S_FMP`    | ✓ (`LO*`)      | ✗              | 6 (no pattern) | **Shown, rank 5**       |

### Complete Examples

#### Example 1: Multiple workflow profiles

Create different profiles for different scenarios:

```toml
[stations.profiles.FIR_Wien]
# Only show stations from FIR Wien
include = ["LO*"]
exclude = ["LON*"]
priority = ["*_FMP", "*_CTR", "LOWW*_APP", "*_APP", "LOWW*_TWR", "*_TWR", "*_GND"]

[stations.profiles.CTR_only]
# Show only center controllers
include = ["*_CTR"]
exclude = ["*_FSS"]
priority = ["LOVV*", "EDMM*"]

[stations.profiles.No_Training]
# Hide common training positions
include = []
exclude = ["*_M_*", "*_X_*", "*_OBS"]
priority = ["*_FMP", "*_CTR", "*_APP", "*_TWR", "*_GND"]
```

#### Example 2: FIR-specific profiles

Create profiles for different FIRs you control in:

```toml
[stations.profiles.LOVV]
include = ["LO*"]
exclude = ["LON*"]
priority = ["LOVV*", "LOWW*"]

[stations.profiles.EDMM]
include = ["EDMM*", "EDDM*"]
exclude = []
priority = ["EDMM*", "EDDM*"]
```

#### Example 3: Role-based profiles

Create profiles based on your controlling position:

```toml
[stations.profiles.TWR]
# When controlling tower, show relevant positions
include = ["LO*"]
exclude = ["LON*"]
priority = ["LOWW*_APP", "LOWW*_TWR", "LOWW*_GND", "LOWW*_DEL"]

[stations.profiles.CTR]
# When controlling center, focus on adjacent centers and approach
include = ["*_CTR", "*_APP"]
exclude = []
priority = ["LOVV*_CTR", "EDMM*_CTR", "*_CTR", "LOWW*_APP", "EDDM*_APP", "*_APP"]
```

### Tips

- Create multiple profiles for different workflows (e.g., "Default", "CTR", "APP")
- Use descriptive profile names that indicate their purpose
- Start with simple patterns and add complexity as needed
- Use `exclude` to refine broad `include` patterns
- Put your most important stations at the top of `priority`
- Leave `include` empty to see everything (filtered only by `exclude`)
- Remember that `exclude` always wins over `include`
- Use `aliases` to customize display names, but keep in mind that your filter patterns must match the **original** callsigns
- Use the `frequencies` option to toggle display of frequencies on your DA keys
- You can switch between profiles in the UI without restarting the application
