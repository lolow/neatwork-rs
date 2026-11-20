from app.neatwork.parsers.parsing_utils import clean_lines, rows_to_dicts, split_csv_row, to_number

def parse_diameters_db(content: str) -> list[dict]:
    rows = clean_lines(content)
    columns = ["size", "SDR", "diam", "cost", "pressure", "type", "rugo"]
    parsed = rows_to_dicts(rows, columns)

    for index, row in enumerate(parsed):
        row.setdefault("ID", f"D{index + 1:03d}")

    return parsed

def parse_orifices_db(content: str) -> list[dict]:
    rows = clean_lines(content)
    result: list[dict] = []

    for index, row in enumerate(rows):
        values = split_csv_row(row)

        if not values:
            continue

        diameter_m = to_number(values[0])
        
        result.append({
"source_index": index + 1,
            "diameter_m": diameter_m,
            "diameter_mm": diameter_m * 1000 if isinstance(diameter_m, (int, float)) else None,
        })

    return result 