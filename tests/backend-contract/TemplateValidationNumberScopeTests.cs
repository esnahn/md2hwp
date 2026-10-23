// SPDX-License-Identifier: AGPL-3.0-only
using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class TemplateValidationNumberScopeTests
{
    internal static void Run()
    {
        var valid = Fixture();
        var original = valid.ToString();
        TemplateValidation.InspectHeadingNumberScopes(valid);
        Check(valid.ToString() == original, "Strict number inspection mutated the source template.");

        foreach (var placement in new[] { "definition", "static", "nested-static", "source", "caption-without-slot" })
        {
            var document = Fixture();
            var orphan = P(Tag("num:heading1"));
            switch (placement)
            {
                case "definition": Find(document, "end:template").AddBeforeSelf(orphan); break;
                case "static": Find(document, "begin:template").AddBeforeSelf(orphan); break;
                case "nested-static":
                    Find(document, "begin:template").AddBeforeSelf(Container("TABLE", orphan)); break;
                case "source":
                    Caption(document, "PICTURE").Add(P(Tag("num:heading1") + Tag("slot:figure.source"))); break;
                case "caption-without-slot": Caption(document, "PICTURE").Add(orphan); break;
            }
            RejectUnchanged(document, placement);
        }

        foreach (var owner in new[] { "PICTURE", "TABLE" })
        foreach (var number in new[] { "num:heading2", "num:heading7", "num:heading" })
        {
            var document = Fixture();
            Caption(document, owner).Element("P")!.Element("TEXT")!.Add(new XElement("CHAR", Tag(number)));
            RejectUnchanged(document, owner + ":" + number);
        }

        foreach (var condition in new[] { "once", "except.once" })
        {
            var document = Fixture();
            Find(document, "end:heading2").AddBeforeSelf(P(Tag("begin:" + condition)),
                P(Tag("num:heading1") + "." + Tag("num:heading2")), P(Tag("end:" + condition)));
            TemplateValidation.InspectHeadingNumberScopes(document);
            document = Fixture();
            Find(document, "end:heading2").AddBeforeSelf(P(Tag("begin:" + condition)),
                P(Tag("num:heading3")), P(Tag("end:" + condition)));
            RejectUnchanged(document, condition);
        }
        var wrongChild = Fixture();
        Find(wrongChild, "end:each.child:heading3").AddBeforeSelf(P(Tag("num:heading4")));
        RejectUnchanged(wrongChild, "unused each.child");
        Console.WriteLine("Strict template number inspection rejects orphan and deeper tags across static content, native captions and unused heading branches.");
    }

    private static XDocument Fixture()
    {
        var document = TemplateCrossReferenceTests.Fixture();
        Find(document, "end:template").AddBeforeSelf(
            P(Tag("begin:heading2")),
            P(Tag("num:heading1") + "." + Tag("num:heading2") + " " + Tag("slot:heading2")),
            P(Tag("begin:each.child:heading3")),
            Container("TABLE", P(Tag("num:heading1") + "." + Tag("num:heading3") + Tag("slot:heading3"))),
            P(Tag("end:each.child:heading3")), P(Tag("end:heading2")),
            P(Tag("num:heading1") + Tag("heading1")),
            NativeCaption("PICTURE", "figure"), NativeCaption("TABLE", "table"));
        return document;
    }

    private static XElement NativeCaption(string owner, string role) => PControl(new XElement(owner,
        new XElement("SHAPEOBJECT", new XElement("CAPTION", new XElement("PARALIST",
            P(Tag("num:heading1") + Tag("slot:" + role + ".caption")))))));
    private static XElement Container(string owner, XElement paragraph) => PControl(new XElement(owner,
        new XElement("ROW", new XElement("CELL", new XElement("PARALIST", paragraph)))));
    private static XElement PControl(XElement control) => new("P", new XElement("TEXT", control));
    private static XElement Caption(XDocument document, string owner) => document.Descendants(owner)
        .Elements("SHAPEOBJECT").Elements("CAPTION").Elements("PARALIST").Single();
    private static XElement Find(XDocument document, string tag) => document.Descendants("P")
        .Single(p => TaggedTemplateBinding.DirectText(p) == Tag(tag));
    private static XElement P(string value) => new("P", new XElement("TEXT", new XAttribute("CharShape", "0"), new XElement("CHAR", value)));
    private static string Tag(string value) => TaggedTemplateBinding.Tag(value);
    private static void Check(bool value, string message) { if (!value) throw new Exception(message); }
    private static void RejectUnchanged(XDocument document, string context)
    {
        var original = document.ToString();
        try { TemplateValidation.InspectHeadingNumberScopes(document); }
        catch (InvalidDataException)
        {
            Check(document.ToString() == original, "Rejected number inspection mutated the source: " + context);
            return;
        }
        throw new Exception("Invalid number scope accepted: " + context);
    }
}
