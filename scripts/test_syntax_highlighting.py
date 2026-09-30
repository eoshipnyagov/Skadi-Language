import json
from pathlib import Path
import re
import unittest

from pygments.token import Keyword, Name, Operator, String
from skadi_docs_lexer import SkadiLexer


ROOT = Path(__file__).resolve().parent.parent
GRAMMAR = json.loads(
    (ROOT / "tools/vscode-skadi-syntax/syntaxes/skadi.tmLanguage.json").read_text(
        encoding="utf-8"
    )
)


def grammar_pattern(name: str) -> str:
    return GRAMMAR["repository"][name]["patterns"][0]["match"]


def docs_tokens(source: str) -> list[tuple[object, str]]:
    return [(token, value) for _, token, value in SkadiLexer().get_tokens_unprocessed(source)]


class SyntaxHighlightingTests(unittest.TestCase):
    def test_vscode_covers_current_lexical_forms(self) -> None:
        char = grammar_pattern("char-literals")
        self.assertIsNotNone(re.fullmatch(char, "'a'"))
        self.assertIsNotNone(re.fullmatch(char, r"'\n'"))
        self.assertIsNone(re.fullmatch(char, "'ab'"))
        self.assertIsNotNone(re.search(grammar_pattern("imports"), 'import "./math.skd"'))

        operators = grammar_pattern("operators")
        for symbol in ("%", "&&", "||", "!", "&", "|", "xor", "div", "mod"):
            with self.subTest(symbol=symbol):
                self.assertIsNotNone(re.fullmatch(operators, symbol))

        self.assertIsNone(re.fullmatch(grammar_pattern("constants"), "null"))

    def test_docs_lexer_covers_same_forms_without_phantom_builtins(self) -> None:
        tokens = docs_tokens("import \"./math.skd\"\nChar c = '\\n'\n")
        self.assertIn((Keyword, "import"), tokens)
        self.assertIn((String.Char, r"'\n'"), tokens)

        tokens = docs_tokens("a += b % c && !d || e xor f\n")
        for symbol in ("+=", "%", "&&", "!", "||", "xor"):
            with self.subTest(symbol=symbol):
                self.assertIn((Operator, symbol), tokens)

        tokens = docs_tokens("print() null fs.read()\n")
        self.assertIn((Name, "print"), tokens)
        self.assertIn((Name, "null"), tokens)
        self.assertNotIn((Name.Builtin.Pseudo, "fs.read"), tokens)


if __name__ == "__main__":
    unittest.main()
