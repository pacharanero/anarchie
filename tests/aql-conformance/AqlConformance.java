// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import org.antlr.v4.runtime.BaseErrorListener;
import org.antlr.v4.runtime.CharStreams;
import org.antlr.v4.runtime.CommonTokenStream;
import org.antlr.v4.runtime.RecognitionException;
import org.antlr.v4.runtime.Recognizer;

/** Runs the official grammar over the repository-owned AQL conformance corpus. */
public final class AqlConformance {
    private AqlConformance() {}

    public static void main(String[] args) throws IOException {
        if (args.length != 2) {
            throw new IllegalArgumentException("usage: AqlConformance <suite-dir> <cases.tsv>");
        }

        Path suiteDirectory = Path.of(args[0]);
        List<String> lines = Files.readAllLines(Path.of(args[1]), StandardCharsets.UTF_8);
        boolean passed = true;

        for (String line : lines) {
            if (line.isBlank() || line.startsWith("#")) {
                continue;
            }

            String[] fields = line.split("\\t", -1);
            if (fields.length != 5) {
                throw new IllegalArgumentException("invalid conformance case: " + line);
            }

            String actual = parse(Files.readString(suiteDirectory.resolve(fields[1]), StandardCharsets.UTF_8));
            System.out.println(fields[0] + "\t" + actual);
            if (!actual.equals(fields[2])) {
                System.err.printf(
                    "%s: official grammar returned %s, expected %s%n", fields[0], actual, fields[2]
                );
                passed = false;
            }
        }

        if (!passed) {
            System.exit(1);
        }
    }

    private static String parse(String query) {
        SyntaxErrors errors = new SyntaxErrors();

        try {
            AqlLexer lexer = new AqlLexer(CharStreams.fromString(query));
            lexer.removeErrorListeners();
            lexer.addErrorListener(errors);

            AqlParser parser = new AqlParser(new CommonTokenStream(lexer));
            parser.removeErrorListeners();
            parser.addErrorListener(errors);
            parser.selectQuery();
        } catch (RuntimeException exception) {
            return "reject";
        }

        return errors.count == 0 ? "accept" : "reject";
    }

    private static final class SyntaxErrors extends BaseErrorListener {
        private int count;

        @Override
        public void syntaxError(
            Recognizer<?, ?> recognizer,
            Object offendingSymbol,
            int line,
            int column,
            String message,
            RecognitionException exception
        ) {
            count++;
        }
    }
}
