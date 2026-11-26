#!/usr/bin/env python3
"""Writes tests/broken/*.puml and tests/broken/expect.tsv.
Each file has exactly one mistake. expect.tsv: name, line, hint substring."""
import os
d = os.path.join(os.path.dirname(__file__), "broken")
os.makedirs(d, exist_ok=True)
cases = []
def case(name, body, line, hint):
    cases.append((name, "@startuml " + name.replace("-", "_") + "\n" + body.strip("\n") + "\n@enduml\n", line, hint))

case("01-typo-if", "start\niff (a · signed in?) then (yes)\n  :b · open the app;\nendif\nstop", 3, "if (")
case("02-typo-endif", "start\nif (a · signed in?) then (yes)\n  :b · open the app;\nendiff\nstop", 5, "endif")
case("03-unclosed-if", "start\nif (a · signed in?) then (yes)\n  :b · open the app;\nstop", 3, "endif")
case("04-unclosed-switch", "start\nswitch (a · which area?)\ncase (people)\n  :b · people;\ncase (workshops)\n  :c · workshops;\nstop", 3, "endswitch")
case("05-unclosed-while", "start\nwhile (a · a step open?) is (yes)\n  :b · do it;\nstop", 3, "endwhile")
case("06-unclosed-repeat", "start\nrepeat\n  :a · enter the code;\nstop", 3, "repeat while")
case("07-unclosed-fork", "start\nfork\n  :a · one;\nfork again\n  :b · two;\nstop", 3, "end fork")
case("08-missing-semicolon-before-stmt", "start\n:a · open the app\nstop", 3, "`;`")
case("09-missing-then", "start\nif (a · signed in?) (yes)\n  :b · open;\nendif\nstop", 3, "then")
case("10-missing-paren", "start\nif a · signed in? then (yes)\n  :b · open;\nendif\nstop", 3, "(")
case("11-unclosed-paren", "start\nif (a · signed in? then (yes)\n  :b · open;\nendif\nstop", 3, ")")
case("12-else-twice", "start\nif (a · ok?) then (yes)\n  :b · go;\nelse (no)\n  :c · stop;\nelse (maybe)\n  :d · wait;\nendif\nstop", 7, "elseif")
case("13-case-outside-switch", "start\ncase (people)\n  :a · people;\nstop", 3, "switch")
case("14-wrong-closer", "start\nwhile (a · open?) is (yes)\n  :b · do;\nendif\nstop", 5, "endwhile")
case("15-backward-outside-repeat", "start\nbackward :a · again;\nstop", 3, "repeat")
case("16-typo-switch", "start\nswich (a · which?)\ncase (x)\n  :b · x;\nendswitch\nstop", 3, "switch")
case("17-typo-repeat-while", "start\nrepeat\n  :a · try;\nrepeat whle (b · done?) is (no) not (yes)\nstop", 5, "repeat while")
case("18-elif", "start\nif (a · ok?) then (yes)\n  :b · go;\nelif (c · other?) then (yes)\n  :d · other;\nendif\nstop", 5, "elseif")
case("19-statement-before-case", "start\nswitch (a · which?)\n:b · oops;\ncase (x)\n  :c · x;\nendswitch\nstop", 4, "case")
case("20-end-if-two-words", "start\nif (a · ok?) then (yes)\n  :b · go;\nend if\nstop", 5, "endif")
case("21-missing-startuml", ":a · open;\nstop", 1, "@startuml")
case("22-missing-enduml-text", "start\n:a · open;\nstop", 4, "@enduml")
case("23-bad-lane", "start\n|trainer\n:a · open;\nstop", 3, "|")
case("24-typo-fork-again", "start\nfork\n  :a · one;\nfork agin\n  :b · two;\nend fork\nstop", 5, "fork again")
case("25-repeat-while-no-condition", "start\nrepeat\n  :a · try;\nrepeat while is (no)\nstop", 5, "(")
case("26-unclosed-note", "start\n:a · open;\nnote right\n  text\nstop", 4, "end note")
case("27-endwhile-orphan", "start\n:a · open;\nendwhile\nstop", 4, "while")
case("28-empty-action", "start\n:;\nstop", 3, "text")

rows = []
for name, text, line, hint in cases:
    if name.startswith("21"):
        text = text.split("\n", 1)[1]
    if name.startswith("22"):
        text = text.replace("@enduml\n", "")
    open(os.path.join(d, name + ".puml"), "w").write(text)
    rows.append((name, line, hint))
with open(os.path.join(d, "expect.tsv"), "w") as f:
    for r in rows:
        f.write("\t".join(str(x) for x in r) + "\n")
print(len(rows), "broken files")
