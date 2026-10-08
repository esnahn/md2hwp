using System.Text.RegularExpressions;
using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class NativeCrossReferenceTests
{
    private const string Source = "MD2HWP_CROSS_REFERENCE_11111111111111111111111111111111";
    private const string NoteSource = "MD2HWP_CROSS_REFERENCE_22222222222222222222222222222222";
    private const string HeadingSource = "MD2HWP_CROSS_REFERENCE_33333333333333333333333333333333";
    private const string LiteralSource = "MD2HWP_CROSS_REFERENCE_99999999999999999999999999999999";
    private const string LiteralNumber = "MD2HWP_NATIVE_NUMBER_REFERENCE_99999999999999999999999999999999";
    private static readonly Regex Number = new("MD2HWP_NATIVE_NUMBER_REFERENCE_[0-9a-f]{32}");
    private static readonly Regex Anchor = new("MD2HWP_NATIVE_HEADING_TARGET_[0-9a-f]{32}");

    internal static void Run()
    {
        static void Check(bool condition, string message) { if (!condition) throw new Exception(message); }
        static void Reject(Action action) { try { action(); } catch (Exception e) when (e is InvalidOperationException or InvalidDataException) { return; } throw new Exception("Expected native-reference rejection."); }
        var fixture = Fixture();
        var original = fixture.ToString();
        var staticRoot = new XElement(Roots(fixture)[0]);
        var verbatimRoot = new XElement(Roots(fixture)[2]);
        var sample = Paragraph("그림 {{md2hwp:num:heading1}}-{{md2hwp:slot:ref.figure.number}}", "2");
        var tableSample = Paragraph("표 {{md2hwp:num:heading1}}-{{md2hwp:slot:ref.table.number}}", "2");
        var headingSample = Paragraph("({{md2hwp:slot:ref.heading.number}})", "1");
        var template = new TemplateCrossReferences(fixture, sample, headingSample, tableSample);
        var prepared = NativeCrossReferences.Prepare(fixture, Plan(), template, Figures(), Headings());
        Check(fixture.ToString() == original, "Reference preparation mutated source HWPML.");
        Check(prepared.Layout.Count == 5, "Duplicate references or nested note references disappeared.");
        var referenceRuns = Roots(prepared.Document)[1].Elements("TEXT").Where(t => Number.Matches(t.Value).Cast<Match>().Any(m => m.Value != LiteralNumber)).ToArray();
        Check(referenceRuns.Any(t => (string?)t.Attribute("CharShape") == "0") && referenceRuns.Any(t => (string?)t.Attribute("CharShape") == "2") &&
            referenceRuns.All(t => (string?)t.Attribute("CharShape") != "1"), "References did not inherit their first source inline format, including emphasis.");
        Check(prepared.Document.Descendants("FOOTNOTE").Descendants("TEXT").Where(t => Number.IsMatch(t.Value)).All(t => (string?)t.Attribute("CharShape") == "0"),
            "Footnote references inherited the reference prototype instead of the note body format.");
        Check(XNode.DeepEquals(staticRoot, Roots(prepared.Document)[0]), "Unrelated mixed static control payload changed.");
        Check(XNode.DeepEquals(verbatimRoot, Roots(prepared.Document)[2]), "Marker-like literal in generated verbatim content changed.");
        Check(Roots(prepared.Document)[3].Elements("TEXT").Count() == 1, "Temporary heading target anchor left an adjacent identical formatting run.");
        Check(prepared.Document.Descendants("P").Count(p => TaggedTemplateBinding.DirectText(p).Contains("그림 8-", StringComparison.Ordinal)) == 2,
            "Figure references used the reader chapter instead of the target chapter.");
        Check(prepared.Document.Descendants("FOOTNOTE").Count() == 1 && prepared.Document.Descendants("FIELDBEGIN").Count(e => (string?)e.Attribute("Command") == "note-date") == 1,
            "Replacing a root reference cloned or swallowed the nested native note subtree.");
        Check(prepared.Document.Descendants("LINEBREAK").Count() == fixture.Descendants("LINEBREAK").Count() && prepared.Document.Descendants("TAB").Count() == fixture.Descendants("TAB").Count(),
            "Mixed CHAR controls were flattened during marker replacement.");
        var native = NativeFields(prepared.Document);
        prepared.Layout.RecordHeadingNumber("fig:heading", "IV.(가)");
        prepared.Layout.Verify(native);
        Check(native.Descendants("FIELDBEGIN").Where(e => (string?)e.Attribute("Type") == "Crossref").Select(e => (string?)e.Attribute("FieldId")).Distinct().Count() == 1,
            "Fixture must exercise legitimate shared native FieldId with distinct control InstId.");
        var normalized = new XDocument(prepared.Document);
        prepared.Layout.NormalizeExpected(normalized, native);
        Check(TemplateRangeStructure.Equivalent(Roots(normalized), Roots(native), normalized, native), "Normalizing only verified generated fields changed other structures.");
        Check(native.Descendants("FIELDBEGIN").Where(e => (string?)e.Attribute("Type") == "Crossref").Count(e => ((string)e.Attribute("Command")!).Contains(";5;1;0;0", StringComparison.Ordinal)) == 1,
            "Outline reference did not target the native outline object number.");
        Check(NativeCrossReferences.Command("501", "figure_number") == "?#501;1;1;0;0" && NativeCrossReferences.Command("510", "heading_number") == "?#510;5;1;0;0", "Documented native commands changed.");
        var tableFixture = Fixture();
        var nativeTable = tableFixture.Descendants("PICTURE").Single(p => (string?)p.Element("SHAPEOBJECT")?.Attribute("InstId") == "101");
        nativeTable.Name = "TABLE";
        nativeTable.Element("SHAPEOBJECT")!.SetAttributeValue("NumberingType", "Table");
        nativeTable.Descendants("AUTONUM").Single().SetAttributeValue("NumberType", "Table");
        PreviewTextRun TableReference(PreviewTextRun run) => run.CrossReference?.Kind == "figure_number"
            ? run with { CrossReference = run.CrossReference with { Kind = "table_number" } }
            : run.Footnote is null ? run : run with { Footnote = run.Footnote with {
                Paragraphs = run.Footnote.Paragraphs.Select(p => p with {
                    Lines = p.Lines.Select(l => l with { Runs = l.Runs.Select(TableReference).ToArray() }).ToArray()
                }).ToArray()
            } };
        var plan = Plan();
        var tablePlan = plan with { Operations = plan.Operations.Select(o => o.Kind == "figure"
            ? o with { Kind = "table", FigureId = null, TableId = o.FigureId }
            : o with { FormattedLines = o.FormattedLines?.Select(line => (IReadOnlyList<PreviewTextRun>)line.Select(TableReference).ToArray()).ToArray() }).ToArray() };
        var tablePrepared = NativeCrossReferences.Prepare(tableFixture, tablePlan, template,
            new Dictionary<string,string>(), new Dictionary<string,IReadOnlyList<string>> { ["fig:heading"] = ["110"] },
            new Dictionary<string,string> { ["한글-대상"] = "101" });
        var tableFields = NativeFields(tablePrepared.Document);
        foreach (var f in tableFields.Descendants("FIELDBEGIN").Where(f => (string?)f.Attribute("Type") == "Crossref"))
            if (((string)f.Attribute("Command")!).Contains(";1;1;0;0", StringComparison.Ordinal))
                f.SetAttributeValue("Command", NativeCrossReferences.Command("501", "table_number") + ";");
        tablePrepared.Layout.RecordHeadingNumber("fig:heading", "IV.(가)");
        tablePrepared.Layout.Verify(tableFields);
        Check(tablePrepared.Layout.Count == prepared.Layout.Count && NativeCrossReferences.Command("501", "table_number") == "?#501;0;1;0;0", "Table field graph lost repeated or nested references.");
        var uncaptioned = new XDocument(tableFixture);
        uncaptioned.Descendants("TABLE").Single(t => t.Element("SHAPEOBJECT") is not null).Element("SHAPEOBJECT")!.Element("CAPTION")!.Remove();
        Reject(() => NativeCrossReferences.Prepare(uncaptioned, tablePlan, template, new Dictionary<string,string>(),
            new Dictionary<string,IReadOnlyList<string>> { ["fig:heading"] = ["110"] }, new Dictionary<string,string> { ["한글-대상"] = "101" }));
        XElement FirstField(XDocument d) => d.Descendants("FIELDBEGIN").First(e => (string?)e.Attribute("Type") == "Crossref");
        void Tamper(Action<XDocument> change)
        {
            var broken = new XDocument(native); change(broken);
            Reject(() => prepared.Layout.Verify(broken));
            var untouched = new XDocument(prepared.Document); var before = untouched.ToString();
            Reject(() => prepared.Layout.NormalizeExpected(untouched, broken));
            Check(untouched.ToString() == before, "Failed graph verification partially normalized expected content.");
        }
        Tamper(d => FirstField(d).SetAttributeValue("Command", "?#90;1;1;0;0;"));
        Tamper(d => FirstField(d).SetAttributeValue("Command", ((string)FirstField(d).Attribute("Command")!).Replace(";1;0;0;", ";1;1;0;", StringComparison.Ordinal)));
        Tamper(d => FirstField(d).SetAttributeValue("Command", ((string)FirstField(d).Attribute("Command")!).Replace(";1;1;0;0;", ";1;0;0;0;", StringComparison.Ordinal)));
        Tamper(d => FirstField(d).SetAttributeValue("Command", (string)FirstField(d).Attribute("Command")! + ";"));
        Tamper(d => FirstField(d).ElementsAfterSelf("CHAR").First().Value = "99");
        Tamper(d => FirstField(d).Parent!.SetAttributeValue("CharShape", "1"));
        Tamper(d => d.Descendants("PICTURE").Single(p => (string?)p.Element("SHAPEOBJECT")?.Attribute("InstId") == "501").Element("SHAPEOBJECT")!.SetAttributeValue("NumberingType", "None"));
        Tamper(d => d.Descendants("AUTONUM").Single(e => (string?)e.Attribute("NumberType") == "Figure").SetAttributeValue("Number", "3"));
        Tamper(d => FirstField(d).ElementsAfterSelf("FIELDEND").First().Remove());
        Tamper(d => FirstField(d).ElementsAfterSelf("FIELDEND").First().SetAttributeValue("FieldId", "999"));
        Tamper(d => FirstField(d).ElementsAfterSelf("FIELDEND").First().SetAttributeValue("Type", "Date"));
        Tamper(d => FirstField(d).SetAttributeValue("Editable", "true"));
        Tamper(d => d.Descendants("FIELDBEGIN").Where(e => (string?)e.Attribute("Type") == "Crossref").Skip(1).First().SetAttributeValue("InstId", (string)FirstField(d).Attribute("InstId")!));
        Tamper(d => d.Descendants("PARASHAPE").Single(e => (string?)e.Attribute("Id") == "1").SetAttributeValue("HeadingType", "Number"));
        var noReferences = Plan() with { Operations = [] };
        var untouchedResult = NativeCrossReferences.Prepare(fixture, noReferences, template, Figures(), Headings());
        Check(untouchedResult.Layout.Count == 0 && XNode.DeepEquals(fixture, untouchedResult.Document), "No-reference preparation changed static IR-marker-like literal or mixed CHAR controls.");
        untouchedResult.Layout.Verify(fixture);
        Reject(() => NativeCrossReferences.Prepare(fixture, Plan(), template, new Dictionary<string, string>(), Headings()));
        var crossing = new XDocument(fixture);
        Roots(crossing)[1].Elements("TEXT").First().Elements("CHAR").Last().Add(new XElement("LINEBREAK"));
        Reject(() => NativeCrossReferences.Prepare(crossing, Plan(), template, Figures(), Headings()));
        var noOutline = new XDocument(fixture);
        noOutline.Descendants("PARASHAPE").Single(e => (string?)e.Attribute("Id") == "1").SetAttributeValue("HeadingType", "None");
        Reject(() => NativeCrossReferences.Prepare(noOutline, Plan(), template, Figures(), Headings()));
        var ambiguous = new XDocument(fixture);
        var duplicate = new XElement(Roots(ambiguous)[3]); duplicate.SetAttributeValue("InstId", "111");
        ambiguous.Descendants("SECTION").Single().Add(duplicate);
        Reject(() => NativeCrossReferences.Prepare(ambiguous, Plan(), template, Figures(), new Dictionary<string, IReadOnlyList<string>> { ["fig:heading"] = new[] { "110", "111" } }));
        var plainCopy = new XElement(Roots(ambiguous)[3]); plainCopy.SetAttributeValue("InstId", "112"); plainCopy.SetAttributeValue("ParaShape", "0");
        var validCopy = new XDocument(fixture); validCopy.Descendants("SECTION").Single().Add(plainCopy);
        var acceptedCopy = NativeCrossReferences.Prepare(validCopy, Plan(), template, Figures(), new Dictionary<string, IReadOnlyList<string>> { ["fig:heading"] = new[] { "110", "112" } });
        Check(acceptedCopy.Layout.Count == 5, "An ordinary decorative title copy was mistaken for an outline target.");
        var captionFixture = Fixture();
        captionFixture.Descendants("CAPTION").Single().Element("PARALIST")!.Add(Paragraph(Source + " " + HeadingSource, "2"));
        var captionPrepared = NativeCrossReferences.Prepare(captionFixture, Plan(), new TemplateCrossReferences(captionFixture, sample, headingSample), Figures(), Headings());
        captionPrepared.Layout.RecordHeadingNumber("fig:heading", "IV.(가)");
        var captionNative = NativeFields(captionPrepared.Document);
        captionPrepared.Layout.Verify(captionNative);
        Check(captionNative.Descendants("CAPTION").Descendants("FIELDBEGIN").Count() == 2, "Native caption references lost their target or formatting.");
        SelectionContracts(Check, Reject);
        for (var level = 1; level <= 6; level++)
        {
            var levelFixture = Fixture();
            levelFixture.Descendants("PARASHAPE").Single(p => (string?)p.Attribute("Id") == "1").SetAttributeValue("Level", level - 1);
            var levelPlan = Plan();
            levelPlan = levelPlan with { Operations = levelPlan.Operations.Select(o => o.HeadingId is null ? o : o with { ParagraphStyle = $"heading{level}" }).ToArray() };
            var levelTemplate = new TemplateCrossReferences(levelFixture, sample, headingSample);
            var levelPrepared = NativeCrossReferences.Prepare(levelFixture, levelPlan, levelTemplate, Figures(), Headings());
            levelPrepared.Layout.RecordHeadingNumber("fig:heading", "IV.(가)");
            var levelNative = NativeFields(levelPrepared.Document);
            levelPrepared.Layout.Verify(levelNative);
            Check(levelNative.Descendants("FIELDBEGIN").Any(e => (string?)e.Attribute("Command") == NativeCrossReferences.Command("510", "heading_number") + ";"),
                $"heading{level} did not retain its native outline reference target.");
        }
        Console.WriteLine("Native figure/outline target graph, mixed controls, nested notes and exact selection contracts passed.");
    }

    private static void SelectionContracts(Action<bool, string> check, Action<Action> reject)
    {
        check(NativeCrossReferences.NormalizeSelectedText("\ufeff" + Source + "\r\n") == Source, "Selection diagnostic normalization failed.");
        check(NativeCrossReferences.NormalizeSelectedText(" " + Source) != Source, "Selection normalization removed literal leading content.");
        var selected = XDocument.Parse("<HWPML><HEAD/><BODY><SECTION><P ParaShape='17'><TEXT CharShape='0'><SECDEF><HEADER><PARALIST><P><TEXT><CHAR>unrelated header</CHAR></TEXT></P></PARALIST></HEADER></SECDEF><COLDEF/><CHAR>" + Source[..25] + "</CHAR></TEXT><TEXT CharShape='1'><CHAR>" + Source[25..] + "</CHAR></TEXT></P></SECTION></BODY></HWPML>");
        NativeCrossReferences.RequireSelectedMarker(selected, Source);
        var prefix = new XDocument(selected); prefix.Descendants("SECTION").Single().Element("P")!.Element("TEXT")!.AddFirst(new XElement("CHAR", "● "));
        reject(() => NativeCrossReferences.RequireSelectedMarker(prefix, Source));
        var control = new XDocument(selected); control.Descendants("SECTION").Single().Element("P")!.Element("TEXT")!.Add(new XElement("AUTONUM", new XAttribute("Number", "1")));
        reject(() => NativeCrossReferences.RequireSelectedMarker(control, Source));
        var line = new XDocument(selected); line.Descendants("SECTION").Single().Element("P")!.Element("TEXT")!.Elements("CHAR").Single().Add(new XElement("LINEBREAK"));
        reject(() => NativeCrossReferences.RequireSelectedMarker(line, Source));
        var extra = new XDocument(selected); extra.Descendants("SECTION").Single().Add(Paragraph(""));
        reject(() => NativeCrossReferences.RequireSelectedMarker(extra, Source));
    }

    private static Dictionary<string, string> Figures() => new(StringComparer.Ordinal) { ["한글-대상"] = "101" };
    private static Dictionary<string, IReadOnlyList<string>> Headings() => new(StringComparer.Ordinal) { ["fig:heading"] = new[] { "110" } };
    private static XElement[] Roots(XDocument document) => AuriMinimalBoxPrototype.RootParagraphs(document).ToArray();
    private static XElement Paragraph(string text, string shape = "0") => new("P", new XAttribute("ParaShape", "0"), new XElement("TEXT", new XAttribute("CharShape", shape), new XElement("CHAR", text)));
    private static IrPreviewPlan Plan()
    {
        var noteContent = new PreviewInlineContent(new[] { new PreviewLine(NoteSource, new[] { new PreviewTextRun(NoteSource, false, false, CrossReference: new("figure_number", "한글-대상")) }) });
        var runs = new[] {
            new PreviewTextRun(Source, false, false, CrossReference: new("figure_number", "한글-대상")),
            new PreviewTextRun(HeadingSource, false, false, CrossReference: new("heading_number", "fig:heading")),
            new PreviewTextRun("note", false, false, new PreviewFootnote(new[] { noteContent })) };
        return new("fixture", "fixture", new PreviewSummary(3, 2, 0, 1, 0), new[] {
            new PreviewOperation("text", "reader", [], FormattedLines: new[] { runs }, Heading1Number: 3),
            new PreviewOperation("code", "verbatim", new[] { LiteralSource, LiteralNumber }),
            new PreviewOperation("text", "target heading", [], ParagraphStyle: "heading2", Heading1Number: 8, HeadingId: "fig:heading"),
            new PreviewOperation("figure", "target picture", [], Heading1Number: 8, FigureId: "한글-대상") }, []);
    }

    private static XDocument Fixture()
    {
        var note = new XElement("FOOTNOTE", new XElement("PARALIST", new XElement("P", new XAttribute("ParaShape", "0"), new XElement("TEXT", new XAttribute("CharShape", "0"),
            new XElement("AUTONUM", new XAttribute("NumberType", "Footnote"), new XAttribute("Number", "1")),
            new XElement("FIELDBEGIN", new XAttribute("Type", "Date"), new XAttribute("InstId", "700"), new XAttribute("Command", "note-date")), new XElement("CHAR", "date"), new XElement("FIELDEND", new XAttribute("Type", "Date")),
            new XElement("CHAR", "lead", new XElement("LINEBREAK"), new XElement("TAB"), NoteSource + " and " + NoteSource)))));
        var reader = new XElement("P", new XAttribute("ParaShape", "0"), new XElement("TEXT", new XAttribute("CharShape", "0"),
            new XElement("FIELDBEGIN", new XAttribute("Type", "Date"), new XAttribute("InstId", "100")), new XElement("CHAR", "date"), new XElement("FIELDEND", new XAttribute("Type", "Date")),
            new XElement("CHAR", "before " + LiteralNumber, new XElement("LINEBREAK"), Source[..25])),
            new XElement("TEXT", new XAttribute("CharShape", "2"), new XElement("CHAR", Source[25..] + " and " + Source + " then " + HeadingSource, new XElement("TAB"), "tail"), note));
        var heading = Paragraph("target heading"); heading.SetAttributeValue("ParaShape", "1"); heading.SetAttributeValue("InstId", "110");
        var definitions = new XElement("HEAD",
            new XElement("PARASHAPELIST",
                new XElement("PARASHAPE", new XAttribute("Id", "0"), new XAttribute("HeadingType", "None")),
                new XElement("PARASHAPE", new XAttribute("Id", "1"), new XAttribute("HeadingType", "Outline"), new XAttribute("Level", "1"))),
            new XElement("CHARSHAPELIST",
                new XElement("CHARSHAPE", new XAttribute("Id", "0")),
                new XElement("CHARSHAPE", new XAttribute("Id", "1"), new XElement("BOLD")),
                new XElement("CHARSHAPE", new XAttribute("Id", "2"), new XElement("ITALIC"))));
        var staticParagraph = new XElement("P", new XElement("TEXT", new XAttribute("CharShape", "0"),
            new XElement("CHAR", "static " + LiteralSource, new XElement("LINEBREAK"), "preserved " + LiteralNumber, new XElement("TAB"), "text"),
            new XElement("PICTURE", new XElement("SHAPEOBJECT", new XAttribute("InstId", "90"), new XAttribute("NumberingType", "None")))));
        var caption = new XElement("CAPTION", new XElement("PARALIST", new XElement("P", new XElement("TEXT", new XAttribute("CharShape", "0"),
            new XElement("AUTONUM", new XAttribute("NumberType", "Figure"), new XAttribute("Number", "2")), new XElement("CHAR", "target picture")))));
        var picture = new XElement("P", new XElement("TEXT", new XAttribute("CharShape", "0"),
            new XElement("PICTURE", new XElement("SHAPEOBJECT", new XAttribute("InstId", "101"), new XAttribute("NumberingType", "Figure"), caption))));
        var verbatim = new XElement("P", new XElement("TEXT", new XAttribute("CharShape", "0"), new XElement("TABLE", new XElement("ROW", new XElement("CELL", new XElement("PARALIST",
            new XElement("P", new XElement("TEXT", new XAttribute("CharShape", "0"), new XElement("CHAR", LiteralSource, new XElement("LINEBREAK"), LiteralNumber, new XElement("TAB"), "literal")))))))));
        return new XDocument(new XElement("HWPML", definitions, new XElement("BODY", new XElement("SECTION", staticParagraph, reader, verbatim, heading, picture))));
    }

    private static XDocument NativeFields(XDocument document)
    {
        var native = new XDocument(document);
        native.Descendants("SHAPEOBJECT").Single(e => (string?)e.Attribute("InstId") == "101").SetAttributeValue("InstId", "501");
        native.Descendants("P").Single(e => (string?)e.Attribute("InstId") == "110").SetAttributeValue("InstId", "510");
        var fieldId = 200;
        foreach (var paragraph in native.Descendants("P").ToArray())
        {
            var direct = TaggedTemplateBinding.DirectText(paragraph);
            if (!Number.Matches(direct).Cast<Match>().Any(m => m.Value != LiteralNumber) && !Anchor.IsMatch(direct)) continue;
            foreach (var character in paragraph.Elements("TEXT").Elements("CHAR").Where(c => c.HasElements).ToArray())
            {
                var pieces = new List<XElement>();
                foreach (var node in character.Nodes().ToArray()) { node.Remove(); pieces.Add(new XElement("CHAR", node)); }
                character.ReplaceWith(pieces);
            }
            foreach (var character in paragraph.Elements("TEXT").Elements("CHAR").Where(c => !c.HasElements).ToArray())
            {
                var text = Anchor.Replace(character.Value, "");
                var matches = Number.Matches(text).Cast<Match>().Where(m => m.Value != LiteralNumber).ToArray();
                if (matches.Length == 0) { character.Value = text; continue; }
                var nodes = new List<XElement>(); var offset = 0;
                foreach (Match match in matches)
                {
                    if (match.Index > offset) nodes.Add(new("CHAR", text[offset..match.Index]));
                    var heading = match.Index > 0 && text[match.Index - 1] == '(';
                    nodes.Add(new("FIELDBEGIN", new XAttribute("Type", "Crossref"), new XAttribute("InstId", ++fieldId), new XAttribute("FieldId", "628650598"), new XAttribute("Editable", "false"), new XAttribute("Dirty", "false"), new XAttribute("Property", "0"), new XAttribute("Command", NativeCrossReferences.Command(heading ? "510" : "501", heading ? "heading_number" : "figure_number") + ";")));
                    nodes.Add(new("CHAR", heading ? "IV.(가)" : "2"));
                    nodes.Add(new("FIELDEND", new XAttribute("Type", "Crossref"), new XAttribute("FieldId", "628650598"), new XAttribute("Editable", "false"), new XAttribute("Property", "0")));
                    offset = match.Index + match.Length;
                }
                if (offset < text.Length) nodes.Add(new("CHAR", text[offset..]));
                character.ReplaceWith(nodes);
            }
            foreach (var empty in paragraph.Elements("TEXT").Elements("CHAR").Where(c => !c.HasElements && c.Value.Length == 0).ToArray()) empty.Remove();
            foreach (var text in paragraph.Elements("TEXT").ToArray())
            {
                if (!text.HasElements) { text.Remove(); continue; }
                if (text.PreviousNode is XElement previous && previous.Name == text.Name &&
                    XNode.DeepEquals(new XElement("TEXT", previous.Attributes()), new XElement("TEXT", text.Attributes())))
                {
                    foreach (var node in text.Nodes().ToArray()) { node.Remove(); previous.Add(node); }
                    text.Remove();
                }
            }
            foreach (var text in paragraph.Elements("TEXT"))
            foreach (var character in text.Elements("CHAR").ToArray())
                if (character.PreviousNode is XElement previous && previous.Name == character.Name)
                {
                    foreach (var node in character.Nodes().ToArray()) { node.Remove(); previous.Add(node); }
                    character.Remove();
                }
        }
        return native;
    }
}
