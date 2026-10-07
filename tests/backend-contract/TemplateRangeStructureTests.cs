using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class TemplateRangeStructureTests
{
    internal static void Run()
    {
        XElement Run(int shape, params object[] nodes) => new("TEXT", new XAttribute("CharShape", shape), nodes);
        var split = new XElement("P", Run(1, new XElement("CHAR", "앞 ")),
            Run(1, new XElement("CHAR", new XElement("LINEBREAK")), new XElement("CHAR", " 뒤")),
            Run(2, new XElement("CHAR", "강조")), Run(1, new XElement("CHAR", new XElement("TAB", new XAttribute("Width", 720)), " 끝")));
        var merged = new XElement("P", Run(1, new XElement("CHAR", "앞 ", new XElement("LINEBREAK"), " 뒤")),
            Run(2, new XElement("CHAR", "강조")), Run(1, new XElement("CHAR", new XElement("TAB", new XAttribute("Width", 720)), " 끝")));
        var original = split.ToString();
        Check(TemplateRangeStructure.Equivalent([split], [merged]), "Identical rich text with different TEXT/CHAR segmentation was rejected.");
        Check(split.ToString() == original, "Comparison normalization modified input XML.");
        foreach (var change in new[] { "break", "tab", "tab-width", "space", "format", "order" })
        {
            var different = new XElement(merged);
            switch (change)
            {
                case "break": different.Descendants("LINEBREAK").Single().Remove(); break;
                case "tab": different.Descendants("TAB").Single().Remove(); break;
                case "tab-width": different.Descendants("TAB").Single().SetAttributeValue("Width", 721); break;
                case "space": different.Elements("TEXT").First().Element("CHAR")!.Nodes().OfType<XText>().First().Value = "앞"; break;
                case "format": different.Elements("TEXT").ElementAt(1).SetAttributeValue("CharShape", 1); break;
                case "order":
                    var character = different.Elements("TEXT").First().Element("CHAR")!;
                    character.ReplaceNodes("앞  뒤", new XElement("LINEBREAK")); break;
            }
            Check(!TemplateRangeStructure.Equivalent([split], [different]), "Comparison accepted a real character/control/format change: " + change);
        }
        var explicitFlow = new XElement("P", Run(1, new XElement("TABLE", new XElement("SHAPEOBJECT",
            new XAttribute("TextFlow", "BothSides"), new XAttribute("TextWrap", "Square"),
            new XElement("SIZE", new XAttribute("Width", 1000))))));
        var omittedFlow = new XElement(explicitFlow);
        omittedFlow.Descendants("SHAPEOBJECT").Single().Attribute("TextFlow")!.Remove();
        Check(TemplateRangeStructure.Equivalent([explicitFlow], [omittedFlow]), "Omitted HWPML default TextFlow was treated as structure loss.");
        var nondefaultFlow = new XElement(explicitFlow);
        nondefaultFlow.Descendants("SHAPEOBJECT").Single().SetAttributeValue("TextFlow", "LeftOnly");
        Check(!TemplateRangeStructure.Equivalent([nondefaultFlow], [omittedFlow]), "A nondefault object text-flow change was ignored.");
        var resized = new XElement(omittedFlow);
        resized.Descendants("SIZE").Single().SetAttributeValue("Width", 999);
        Check(!TemplateRangeStructure.Equivalent([explicitFlow], [resized]), "Object geometry loss was ignored.");
        Console.WriteLine("Native TEXT/CHAR segmentation and default TextFlow compare semantically while content, controls and geometry remain strict.");
    }

    private static void Check(bool value, string message) { if (!value) throw new Exception(message); }
}
