using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class TemplateCrossReferenceTests
{
    internal static XDocument Fixture()
    {
        var roots = new List<XElement> { P("before"), P(Tag("begin:template")), P("keep") };
        foreach (var role in new[] { "ref.figure.number", "ref.table.number" })
            roots.AddRange([P(Tag("begin:" + role)), P("object " + TemplateHeadingNumbers.Tag + "-" + Tag("slot:" + role)), P(Tag("end:" + role))]);
        for (var level = 1; level <= 6; level++)
        {
            var role = TemplateCrossReferences.HeadingRole(level);
            var number = string.Join(".", Enumerable.Range(1, level).Select(index => Tag($"num:heading{index}")));
            roots.AddRange([P(Tag("begin:" + role)), P(level == 1 ? "제" + number + "장" : number + (level == 2 ? "절" : "항")), P(Tag("end:" + role))]);
        }
        roots.AddRange([P(Tag("end:template")), P("after")]);
        return new(new XElement("HWPML", new XElement("HEAD",
            new XElement("PARASHAPELIST", new XAttribute("Count", 1), new XElement("PARASHAPE", new XAttribute("Id", 0))),
            new XElement("CHARSHAPELIST", new XAttribute("Count", 1), new XElement("CHARSHAPE", new XAttribute("Id", 0)))),
            new XElement("BODY", new XElement("SECTION", roots))));
    }
    internal static void Run()
    {
        var source = Fixture();
        var original = source.ToString();
        var binding = TemplateCrossReferences.Lower(source);
        Check(source.ToString() == original, "Reference lowering mutated source.");
        Check(AuriMinimalBoxPrototype.RootParagraphs(binding.Document).Select(TaggedTemplateBinding.DirectText)
            .SequenceEqual(new[] { "before", Tag("begin:template"), "keep", Tag("end:template"), "after" }), "Reference lowering removed unrelated roots.");
        var numbers = new[] { 3, 2, 4, 5, 6, 7 };
        Check(binding.Layout.HeadingText(1, numbers) == "제3장" && binding.Layout.HeadingText(2, numbers) == "3.2절" &&
            binding.Layout.HeadingText(6, numbers) == "3.2.4.5.6.7항", "Heading level wording or hierarchy was lost.");
        var prepared = TemplateCrossReferences.Lower(TemplateHeadingNumbers.Prepare(source));
        Check(prepared.Layout.HeadingText(2, numbers) == "3.2절", "Prepared heading1 number marker did not bind to the target chapter.");
        var destination = Fixture();
        var context = new XElement("TEXT", new XAttribute("CharShape", 0), new XAttribute("Custom", "emphasis"));
        var fragments = binding.Layout.CreateFigureNumberFragments(destination, 8, "NATIVE_MARKER", context);
        Check(string.Concat(fragments.Select(fragment => fragment.Value)) == "object 8-NATIVE_MARKER" &&
            fragments.All(fragment => XNode.DeepEquals(new XElement("TEXT", fragment.Attributes()), context)), "Native object references lost wording or source formatting.");
        foreach (var role in new[] { "ref.figure.number", "ref.table.number" }.Concat(Enumerable.Range(1, 6).Select(TemplateCrossReferences.HeadingRole)))
        {
            var missing = Fixture(); Find(missing, "end:" + role).Remove(); Reject(() => TemplateCrossReferences.Lower(missing));
            var duplicate = Fixture(); Find(duplicate, "begin:" + role).AddBeforeSelf(P(Tag("begin:" + role))); Reject(() => TemplateCrossReferences.Lower(duplicate));
            var extra = Fixture(); Find(extra, "begin:" + role).AddAfterSelf(P("extra")); Reject(() => TemplateCrossReferences.Lower(extra));
            var outside = Fixture(); Find(outside, "end:template").AddAfterSelf(P(Tag("slot:" + role))); Reject(() => TemplateCrossReferences.Lower(outside));
            foreach (var control in new[] { "TAB", "LINEBREAK", "AUTONUM", "SECDEF", "TABLE", "PICTURE", "FOOTNOTE" })
            {
                var bad = Fixture(); Find(bad, "begin:" + role).ElementsAfterSelf("P").First().Element("TEXT")!.Add(new XElement(control));
                Reject(() => TemplateCrossReferences.Lower(bad));
            }
            foreach (var attribute in new[] { "PageBreak", "ColumnBreak" })
            {
                var bad = Fixture(); Find(bad, "begin:" + role).ElementsAfterSelf("P").First().SetAttributeValue(attribute, "true");
                Reject(() => TemplateCrossReferences.Lower(bad));
            }
        }
        var obsolete = Fixture(); Find(obsolete, "end:template").AddBeforeSelf(P(Tag("begin:ref.heading.number")));
        Reject(() => TemplateCrossReferences.Lower(obsolete));
        var missingNumber = Fixture(); Find(missingNumber, "begin:ref.heading2.number").ElementsAfterSelf("P").First().ReplaceNodes(new XElement("TEXT", new XAttribute("CharShape", 0), new XElement("CHAR", "section")));
        Reject(() => TemplateCrossReferences.Lower(missingNumber));
        var wrongLevel = Fixture(); Find(wrongLevel, "begin:ref.heading2.number").ElementsAfterSelf("P").First().Element("TEXT")!.Add(new XElement("CHAR", Tag("num:heading3")));
        Reject(() => TemplateCrossReferences.Lower(wrongLevel));
        Console.WriteLine("Heading1 through heading6 reference blocks, numbering tags, object-field formatting and declaration errors passed.");
    }
    private static XElement P(string text) => new("P", new XAttribute("ParaShape", 0), new XElement("TEXT", new XAttribute("CharShape", 0), new XElement("CHAR", text)));
    private static string Tag(string text) => TaggedTemplateBinding.Tag(text);
    private static XElement Find(XDocument document, string tag) => AuriMinimalBoxPrototype.RootParagraphs(document).Single(p => TaggedTemplateBinding.DirectText(p) == Tag(tag));
    private static void Check(bool value, string message) { if (!value) throw new Exception(message); }
    private static void Reject(Action action) { try { action(); } catch (InvalidDataException) { return; } throw new Exception("Expected prototype rejection."); }
}
