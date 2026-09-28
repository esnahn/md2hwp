using System.Globalization;
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal sealed record TemplateCreationResult(string Output, string Contract, bool Reopened);

internal static partial class HancomPreviewWriter
{
    public static TemplateCreationResult CreateDefaultTemplate(string outputPath)
    {
        var output = ValidateNewHwpPath(outputPath, "Template");
        EnsureInteractiveContext();
        EnsureNoExistingHwpProcess();
        var module = SecurityModuleRegistration.ReadAndValidate(Directory.GetCurrentDirectory());
        var temporary = Path.Combine(Path.GetDirectoryName(output)!, $".md2hwp-template-{Guid.NewGuid():N}.hwp");
        try
        {
            WithHwp(module, hwp =>
            {
                // A newly created Hancom document is the only formatting source.
                // No shipped or user-authored template is opened or embedded.
                XDocument blank = HwpMarkup.Parse((string)hwp.GetTextFile("HWPML2X", ""));
                var originalRoots = AuriMinimalBoxPrototype.RootParagraphs(blank);
                if (originalRoots.Count != 1 || originalRoots[0].Value.Length != 0 ||
                    blank.Descendants().Any(e => e.Name.LocalName is "TABLE" or "PICTURE" or "HEADER" or "FOOTER"))
                    throw new InvalidOperationException("Expected a fresh empty Hancom document.");

                // Obtain native default table borders, cell margins and caption layout.
                _ = hwp.HAction.GetDefault("TableCreate", hwp.HParameterSet.HTableCreation.HSet);
                hwp.HParameterSet.HTableCreation.Rows = 1;
                hwp.HParameterSet.HTableCreation.Cols = 1;
                if (!(bool)hwp.HAction.Execute("TableCreate", hwp.HParameterSet.HTableCreation.HSet))
                    throw new InvalidOperationException("Could not create the default table.");
                Run(hwp, "ShapeObjAttachCaption");
                XDocument seed = HwpMarkup.Parse((string)hwp.GetTextFile("HWPML2X", ""));
                var authored = BuildDefaultDeclarations(blank, seed);
                _ = hwp.Clear(1);
                var xml = "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>" + authored.ToString(SaveOptions.DisableFormatting);
                // Unlike SaveAs, SetTextFile returns an integer status (1 = success).
                object imported = hwp.SetTextFile(xml, "HWPML2X", "");
                if (imported is not int { } status || status != 1)
                    throw new InvalidOperationException($"Could not import the default template declarations: {imported} ({imported?.GetType().Name}).");
                if (!IndicatesSuccess(hwp.SaveAs(temporary, "HWP", "")))
                    throw new InvalidOperationException("Could not save the default template.");
                CloseDocument(hwp);
                Open(hwp, temporary, false);
                XDocument reopened = HwpMarkup.Parse((string)hwp.GetTextFile("HWPML2X", ""));
                var binding = TaggedTemplateBinding.Read(reopened, temporary);
                var styles = AuriPreviewStyleBindings.BindDocument(reopened, binding.Profile);
                _ = AuriMinimalBoxPrototype.Bind(hwp, styles);
                _ = AuriMinimalCaptionPrototype.Bind(hwp, styles);
                return true;
            });
            // Never replace an edited template, including one created during generation.
            File.Move(temporary, output);
            return new(output, "minimal-1", true);
        }
        finally { if (File.Exists(temporary)) File.Delete(temporary); }
    }

    internal static XDocument BuildDefaultDeclarations(XDocument blank, XDocument seed)
    {
        var result = new XDocument(seed);
        var section = result.Descendants().Single(e => e.Name.LocalName == "SECTION");
        var blankRoot = AuriMinimalBoxPrototype.RootParagraphs(blank).Single();
        var defaultId = (string?)blankRoot.Attribute("Style") ?? throw new InvalidDataException("Missing default style.");
        var baseStyle = blank.Descendants().Single(e => e.Name.LocalName == "STYLE" && (string?)e.Attribute("Id") == defaultId);
        var paragraphShape = (string)baseStyle.Attribute("ParaShape")!;
        var charShape = (string)baseStyle.Attribute("CharShape")!;
        var styles = result.Descendants().Where(e => e.Name.LocalName == "STYLE").ToArray();
        var styleList = styles[0].Parent!;
        var nextId = styles.Max(e => (int)e.Attribute("Id")!) + 1;
        var roles = new[] { "body", "heading.1", "heading.2", "heading.3", "heading.4", "heading.5", "heading.6", "block.box", "box.source", "figure.caption", "figure.source", "reset" };
        var ids = new Dictionary<string, int>(StringComparer.Ordinal);
        foreach (var role in roles)
        {
            var id = nextId++;
            ids.Add(role, id);
            var style = new XElement(baseStyle);
            style.SetAttributeValue("Id", id);
            style.SetAttributeValue("Name", "md2hwp." + role);
            style.SetAttributeValue("EngName", "md2hwp." + role);
            style.SetAttributeValue("NextStyle", ids["body"]);
            styleList.Add(style);
        }
        styleList.SetAttributeValue("Count", styleList.Elements().Count());
        XElement Paragraph(string role, params object[] contents) =>
            new("P", new XAttribute("ParaShape", paragraphShape), new XAttribute("Style", ids[role]),
                new XElement("TEXT", new XAttribute("CharShape", charShape), contents));
        XElement TextParagraph(string role, string value) => Paragraph(role, new XElement("CHAR", value));
        XElement Declaration(string value, string role = "body") => TextParagraph(role, TaggedTemplateBinding.Tag(value));

        var table = new XElement(result.Descendants().Single(e => e.Name.LocalName == "TABLE"));
        var shape = table.Elements().Single(e => e.Name.LocalName == "SHAPEOBJECT");
        shape.SetAttributeValue("NumberingType", "None");
        shape.Elements().Single(e => e.Name.LocalName == "POSITION").SetAttributeValue("TreatAsChar", "true");
        var page = blank.Descendants().Single(e => e.Name.LocalName == "PAGEDEF");
        var margin = page.Elements().Single(e => e.Name.LocalName == "PAGEMARGIN");
        var width = (int)page.Attribute("Width")! - (int)margin.Attribute("Left")! - (int)margin.Attribute("Right")! - (int)margin.Attribute("Gutter")!;
        if (width <= 0) throw new InvalidDataException("The default page has no usable width.");
        shape.Elements().Single(e => e.Name.LocalName == "SIZE").SetAttributeValue("Width", width);
        var cell = table.Descendants().Single(e => e.Name.LocalName == "CELL");
        cell.SetAttributeValue("Width", width);
        cell.Elements().Single(e => e.Name.LocalName == "PARALIST").ReplaceNodes(Declaration("slot:box.content", "block.box"));
        var caption = shape.Elements().Single(e => e.Name.LocalName == "CAPTION");
        caption.SetAttributeValue("LastWidth", width);
        caption.Elements().Single(e => e.Name.LocalName == "PARALIST").ReplaceNodes(TextParagraph("box.source", "출처: " + TaggedTemplateBinding.Tag("slot:box.source")));

        var figureNumber = new XElement("AUTONUM", new XAttribute("Number", 1), new XAttribute("NumberType", "Figure"),
            new XElement("AUTONUMFORMAT", new XAttribute("Superscript", "false"), new XAttribute("Type", "Digit")));
        var figureCaption = Paragraph("figure.caption", new XElement("CHAR", "[그림 "), figureNumber,
            new XElement("CHAR", "] " + TaggedTemplateBinding.Tag("slot:figure.caption")));
        // Retain only the empty first paragraph's native section/page definition.
        var first = new XElement(blankRoot);
        first.SetAttributeValue("Style", ids["body"]);
        var roots = new List<XElement> { first, Declaration("begin:samples"), Declaration("contract:minimal-1") };
        var figureWidth = Math.Min(142, width * 25.4 / 7200).ToString("0.###", CultureInfo.InvariantCulture);
        roots.AddRange(new[] { Declaration("figure.max-width-mm:" + figureWidth), Declaration("lists.max-depth:6"),
            Declaration("lists.indent-hwp:1000") });
        foreach (var role in roles.Where(r => r == "body" || r.StartsWith("heading.", StringComparison.Ordinal) || r == "reset"))
            roots.Add(Declaration(role, role));
        roots.AddRange(new[] { Declaration("begin:block.box"), Paragraph("body", table), Declaration("end:block.box"),
            Declaration("begin:figure"), Declaration("slot:figure.image"), figureCaption,
            TextParagraph("figure.source", "출처: " + TaggedTemplateBinding.Tag("slot:figure.source")), Declaration("end:figure"),
            Declaration("end:samples"), Declaration("content"), TextParagraph("body", "") });
        section.ReplaceNodes(roots);
        return result;
    }
}
