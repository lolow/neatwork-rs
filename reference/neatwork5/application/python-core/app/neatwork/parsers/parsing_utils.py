from typing import Any

def clean_lines(content: str) -> list[str]:
    return [
        line.strip().lstrip("\ufeff") #remove utf-8 bom or any whitespace at the begining of the line
        for line in content.splitlines() #split the content into lines
        if line.strip() and not line.strip().startswith("#") #remove empty lines and comments
    ]

def normalize_section_name(value: str) -> str:
    return (
        value.strip() #remove leading and trailing whitespaces
        .lstrip("\ufeff") #remove utf-8 bom or any whitespace at the begining of the line
        .lower() #convert to lowercase
        .replace("_", " ") #replace underscores with spaces
        .replace("-", " ") #replace hyphens with spaces
    )


def get_section(lines: list[str], section_name: str) -> list[str]:
    expected = normalize_section_name(section_name)
    start = -1

    for index, line in enumerate(lines):
        stripped = line.strip().lstrip("\ufeff")

        if not stripped.startswith("!"):
            continue

        current = normalize_section_name(stripped[1:])

        if current == expected:
            start = index + 1
            continue

        if start != -1:
            return lines[start:index]

    return lines[start:] if start != -1 else []

def split_csv_row(row: str) -> list[str]:
    return [value.strip() for value in row.split(",")]

def to_number(value: str) -> Any:
    if value is None:
        return None

    text = str(value).strip()
    if text == "":
        return ""

    try:
        number = float(text)
        return int(number) if number.is_integer() else number
    except ValueError:
        return text

def rows_to_dicts(rows: list[str], columns: list[str]) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []

    for row in rows:
        values = split_csv_row(row)
        if len(values) < len(columns):
            continue
    
        result.append({
            column: to_number(values[index])
            for index, column in enumerate(columns)
        })
    
    return result

def parse_params(rows: list[str]) -> dict[str, Any]:
    params: dict[str, Any] = {}

    for row in rows:
        values = split_csv_row(row)
        if len(values) >= 2:
            params[values[0]] = to_number(values[1])

    return params

def node_type_from_status(status: int | float | str) -> str:
    value = int(float(status))

    if value == 0:
        return "RESERVOIR"
    if value == 2:
        return "FAUCET"
    
    return "JUNCTION"

def safe_float(value: Any, default: float = 0.0) -> float:
    try:
        return float(value)
    except (TypeError, ValueError):
        return default

