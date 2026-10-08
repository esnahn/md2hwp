using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class HeadingTemplateNumbersTests
{
    internal static void Run()
    {
        static string Tag(string value) => TaggedTemplateBinding.Tag(value);
        static XElement P(string text, string shape = "0") => new("P", new XAttribute("Style", "0"), new XAttribute("ParaShape", "0"), new XElement("TEXT", new XAttribute("CharShape", shape), new XElement("CHAR", text)));
        static XDocument Doc(IEnumerable<XElement> paragraphs) => new(new XElement("HWPML",
            new XElement("HEAD", new XElement("PARASHAPELIST", new XElement("PARASHAPE", new XAttribute("Id", "0"))),
                new XElement("CHARSHAPELIST", new XElement("CHARSHAPE", new XAttribute("Id", "0")), new XElement("CHARSHAPE", new XAttribute("Id", "1"), new XElement("ITALIC")))),
            new XElement("BODY", new XElement("SECTION", paragraphs))));
        static PreviewOperation H(int level, string title, int chapter) => new("text", title, [title], ParagraphStyle: $"heading{level}", Heading1Number: chapter);
        var operations = new[] { H(1, "chapter3", 3), H(2, "first", 3), H(3, "nestedA", 3), H(2, "second", 3), H(3, "nestedB", 3), H(3, "nestedC", 3), H(1, "chapter4", 4), H(2, "reset", 4) };
        var plan = new IrPreviewPlan("fixture", "fixture", new(8, 8, 0, 0, 0), operations, []);
        var prefix = P(Tag("num:heading1") + "." + Tag("num:heading2") + " ");
        prefix.Add(new XElement("TEXT", new XAttribute("CharShape", "1"), new XElement("CHAR", "{{md2hwp:slot:head")),
            new XElement("TEXT", new XAttribute("CharShape", "0"), new XElement("CHAR", "ing2}}" + " 뒤")));
        var range = new[] { P(Tag("slot:heading1")), P(Tag("begin:each.child:heading2")), prefix,
            P(Tag("begin:each.child:heading3")), P(Tag("num:heading1") + "." + Tag("num:heading2") + "." + Tag("num:heading3") + " " + Tag("slot:heading3")),
            P(Tag("end:each.child:heading3")), P(Tag("end:each.child:heading2")) };
        var fixture = TemplateHeadingNumbers.Prepare(Doc(new[] { P(Tag("begin:template")), P(Tag("begin:heading1")) }.Concat(range).Concat(new[] { P(Tag("end:heading1")), P(Tag("end:template")) })));
        var before = fixture.ToString();
        var layout = TemplateHeadingBlocks.Lower(fixture).Layout;
        var rendered = Doc(operations.Select(operation => P(operation.Lines[0])));
        var result = layout.Attach(rendered, plan, 0);
        var text = AuriMinimalBoxPrototype.RootParagraphs(result).Select(TaggedTemplateBinding.DirectText).ToArray();
        Check(text.Take(6).SequenceEqual(new[] { "chapter3", "3.1 first 뒤", "3.1.1 nestedA", "3.2 second 뒤", "3.2.1 nestedB", "3.2.2 nestedC" }), "Nested child scopes used parent or repetition counters.");
        Check(text.Contains("4.1 reset 뒤") && fixture.ToString() == before, "Chapter reset or source immutability failed.");
        var child = AuriMinimalBoxPrototype.RootParagraphs(result).First(paragraph => TaggedTemplateBinding.DirectText(paragraph) == "3.1 first 뒤");
        Check((string?)child.Elements("TEXT").First().Attribute("CharShape") == "0" && (string?)child.Elements("TEXT").First(t => t.Value == "first").Attribute("CharShape") == "1", "Number prefix or split title-slot formatting changed.");
        TemplateHeadingNumbers.RequireResolved(result);

        var plain = TemplateHeadingNumbers.Prepare(Doc([P(Tag("begin:template")), P(Tag("num:heading1") + "." + Tag("num:heading2") + " " + Tag("heading2")), P(Tag("end:template"))]));
        var plainLayout = TemplateHeadingBlocks.Lower(plain).Layout;
        var plainResult = plainLayout.Attach(rendered, plan, 0);
        Check(AuriMinimalBoxPrototype.RootParagraphs(plainResult).Any(paragraph => TaggedTemplateBinding.DirectText(paragraph) == "3.2 second"), "Numbered one-paragraph heading declaration was not expanded.");
        var skip = P(Tag("num:heading1") + "." + Tag("num:heading2") + "." + Tag("num:heading3"));
        TemplateHeadingNumbers.FillParagraph(skip, 3, [0, 0, 1, 0, 0, 0]);
        Check(TaggedTemplateBinding.DirectText(skip) == "0.0.1", "Skipped parents did not match reference counters.");
        Reject(() => TemplateHeadingEach.Validate([P(Tag("num:heading3") + Tag("slot:heading2"))], 2));
        Reject(() => TemplateHeadingEach.Validate([P(Tag("slot:heading2") + Tag("slot:heading2"))], 2));
        Reject(() => TemplateHeadingEach.Validate([P(Tag("num:heading7") + Tag("slot:heading2"))], 2));
        var orphan = TemplateHeadingNumbers.Prepare(Doc([P(Tag("num:heading2"))]));
        Reject(() => TemplateHeadingNumbers.RequireResolved(orphan));
        TemplateHeadingNumbers.RequireResolved(Doc([P(Tag("num:heading2"))])); // Literal manuscript/metadata text is not a prepared template slot.
        var preparedRefs = TemplateCrossReferences.Lower(TemplateHeadingNumbers.Prepare(TemplateCrossReferenceTests.Fixture())).Layout;
        Check(preparedRefs.HeadingText(6, [3, 2, 1, 1, 1, 1]) == "3.2.1.1.1.1항", "Prepared reference prototypes diverged from title counters.");
        Console.WriteLine("All heading and nested-child number scopes, inline title affixes, format preservation and orphan checks passed.");
    }
    private static void Check(bool value, string message) { if (!value) throw new Exception(message); }
    private static void Reject(Action action) { try { action(); } catch (InvalidDataException) { return; } throw new Exception("Expected rejection."); }
}
