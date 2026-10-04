using System.Globalization;
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal sealed record TemplateCreationResult(string Output, string IrVersion, bool Reopened);

internal static partial class HancomPreviewWriter
{
    private const string DefaultSourcePrefix = "출처: ";

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

                // Native list samples start from Hancom's fresh-document defaults.
                foreach (var (kind, action) in new[] { ("bullet", "PutBullet"), ("ordered", "PutParaNumber") })
                {
                    Run(hwp, action);
                    InsertText(hwp, TaggedTemplateBinding.Tag("list." + kind));
                    Run(hwp, "BreakPara");
                    _ = hwp.HAction.GetDefault("ParagraphShape", hwp.HParameterSet.HParaShape.HSet);
                    hwp.HParameterSet.HParaShape.HeadingType = 0;
                    hwp.HParameterSet.HParaShape.Level = 0;
                    if (!(bool)hwp.HAction.Execute("ParagraphShape", hwp.HParameterSet.HParaShape.HSet))
                        throw new InvalidOperationException("Could not end the native list sample.");
                }

                // Obtain native default table borders, cell margins and caption layout.
                _ = hwp.HAction.GetDefault("TableCreate", hwp.HParameterSet.HTableCreation.HSet);
                hwp.HParameterSet.HTableCreation.Rows = 1;
                hwp.HParameterSet.HTableCreation.Cols = 1;
                if (!(bool)hwp.HAction.Execute("TableCreate", hwp.HParameterSet.HTableCreation.HSet))
                    throw new InvalidOperationException("Could not create the default table.");
                Run(hwp, "ShapeObjAttachCaption");
                Run(hwp, "Cancel"); Run(hwp, "MoveDocEnd"); Run(hwp, "BreakPara");
                var sample = Path.Combine(Path.GetTempPath(), $"md2hwp-sample-{Guid.NewGuid():N}.png");
                try
                {
                    File.WriteAllBytes(sample, Convert.FromBase64String("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGO4e/cuAAUyApjDDfqPAAAAAElFTkSuQmCC"));
                    if (!IndicatesSuccess(hwp.InsertPicture(sample, true, 1, false, false, 0, 142.0, 12.0)))
                        throw new InvalidOperationException("Could not create sample picture.");
                    Run(hwp, "MoveParaBegin"); Run(hwp, "SelectCtrlFront"); Run(hwp, "ShapeObjAttachCaption");
                }
                finally { if (File.Exists(sample)) File.Delete(sample); }
                XDocument seed = HwpMarkup.Parse((string)hwp.GetTextFile("HWPML2X", ""));
                var authored = BuildDefaultDeclarations(blank, seed);
                _ = hwp.Clear(1);
                var xml = "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>" + authored.ToString(SaveOptions.DisableFormatting);
                // Unlike SaveAs, SetTextFile returns an integer status (1 = success).
                object imported = hwp.SetTextFile(xml, "HWPML2X", "");
                if (imported is not int { } status || status != 1)
                    throw new InvalidOperationException($"Could not import the default template declarations: {imported} ({imported?.GetType().Name}).");
                // Locate each native caption's slot. Hancom MoveLeft leaves the
                // selection one character before its start; MoveRight returns to
                // the tag boundary without depending on the prefix length.
                foreach (var role in new[] { "box.source", "figure.source" })
                {
                    Run(hwp, "MoveDocBegin");
                    FindTaggedText(hwp, TaggedTemplateBinding.Tag("slot:" + role));
                    Run(hwp, "MoveLeft");
                    Run(hwp, "MoveRight");
                    Run(hwp, "ParagraphShapeIndentAtCaret");
                }
                XDocument adjusted = HwpMarkup.Parse((string)hwp.GetTextFile("HWPML2X", ""));
                foreach (var role in new[] { "box.source", "figure.source" })
                {
                    var slot = TaggedTemplateBinding.Tag("slot:" + role);
                    var measuredParagraph = adjusted.Descendants("P").Single(p =>
                        TaggedTemplateBinding.DirectText(p).Contains(slot, StringComparison.Ordinal));
                    var measuredShape = adjusted.Descendants("PARASHAPE").Single(e =>
                        (string?)e.Attribute("Id") == (string?)measuredParagraph.Attribute("ParaShape"));
                    var indent = (int)measuredShape.Element("PARAMARGIN")!.Attribute("Indent")!;
                    if (indent >= 0)
                        throw new InvalidOperationException($"Hancom did not calculate the hanging indent before slot:{role}.");
                    // Transfer the calculated indent into the authored document;
                    // exported table IDs may be normalized by Hancom.
                    var paragraph = authored.Descendants("P").Single(p =>
                        TaggedTemplateBinding.DirectText(p).Contains(slot, StringComparison.Ordinal));
                    var shape = new XElement(authored.Descendants("PARASHAPE").Single(e =>
                        (string?)e.Attribute("Id") == (string?)paragraph.Attribute("ParaShape")));
                    var shapeList = authored.Descendants("PARASHAPE").First().Parent!;
                    var shapeId = shapeList.Elements().Max(e => (int)e.Attribute("Id")!) + 1;
                    shape.SetAttributeValue("Id", shapeId);
                    shape.Element("PARAMARGIN")!.SetAttributeValue("Indent", indent);
                    shapeList.Add(shape);
                    shapeList.SetAttributeValue("Count", shapeList.Elements().Count());
                    paragraph.SetAttributeValue("ParaShape", shapeId);
                    authored.Descendants("STYLE").Single(e => (string?)e.Attribute("Name") == "md2hwp." + role)
                        .SetAttributeValue("ParaShape", shapeId);
                }
                // Keep style-owned shapes before disposable prototype-only shapes.
                // Hancom removes unused list shapes on save and renumbers later IDs.
                var styleShapes = authored.Descendants("STYLE").Select(e => (string)e.Attribute("ParaShape")!).ToHashSet();
                var allShapes = authored.Descendants("PARASHAPE").OrderBy(e =>
                    styleShapes.Contains((string)e.Attribute("Id")!) ? 0 : 1).ToArray();
                var shapeIds = allShapes.Select((e, i) => (Old: (string)e.Attribute("Id")!, New: i))
                    .ToDictionary(e => e.Old, e => e.New);
                var definitions = allShapes[0].Parent!;
                foreach (var reference in authored.Descendants().Attributes("ParaShape"))
                    reference.Value = shapeIds[reference.Value].ToString(CultureInfo.InvariantCulture);
                foreach (var shape in allShapes)
                    shape.SetAttributeValue("Id", shapeIds[(string)shape.Attribute("Id")!]);
                definitions.ReplaceNodes(allShapes);
                // Keep named styles consistent with the independently adjusted samples.
                _ = hwp.Clear(1);
                xml = "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>" + authored.ToString(SaveOptions.DisableFormatting);
                imported = hwp.SetTextFile(xml, "HWPML2X", "");
                if (imported is not int applied || applied != 1)
                    throw new InvalidOperationException("Could not import the adjusted source styles.");
                if (!IndicatesSuccess(hwp.SaveAs(temporary, "HWP", "")))
                    throw new InvalidOperationException("Could not save the default template.");
                CloseDocument(hwp);
                Open(hwp, temporary, false);
                XDocument reopened = HwpMarkup.Parse((string)hwp.GetTextFile("HWPML2X", ""));
                var binding = TaggedTemplateBinding.Read(reopened, temporary);
                _ = binding.Profile; // Read validates the native sample and all lowered roles.
                foreach (var name in new[] { "md2hwp.box.source", "md2hwp.figure.source" })
                {
                    int SourceIndent(XDocument doc)
                    {
                        var style = doc.Descendants("STYLE").Single(e => (string?)e.Attribute("Name") == name);
                        var shape = doc.Descendants("PARASHAPE").Single(e => (string?)e.Attribute("Id") == (string?)style.Attribute("ParaShape"));
                        return (int)shape.Element("PARAMARGIN")!.Attribute("Indent")!;
                    }
                    if (SourceIndent(authored) >= 0 || SourceIndent(authored) != SourceIndent(reopened))
                        throw new InvalidOperationException("The measured source hanging indent was not preserved on save.");
                }
                return true;
            });
            // Never replace an edited template, including one created during generation.
            File.Move(temporary, output);
            return new(output, IrContract.Version, true);
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
        var roles = new[] { "body", "heading1", "heading2", "heading3", "heading4", "heading5", "heading6", "footnote", "block.box", "box.title", "box.source", "figure.caption", "figure.source", "ref.figure.number", "ref.heading.number", "reset" };
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
        cell.Elements().Single(e => e.Name.LocalName == "PARALIST").ReplaceNodes(Declaration("slot:box.title", "box.title"), Declaration("slot:box.content", "block.box"));
        var caption = shape.Elements().Single(e => e.Name.LocalName == "CAPTION");
        caption.SetAttributeValue("LastWidth", width);
        caption.Elements().Single(e => e.Name.LocalName == "PARALIST").ReplaceNodes(TextParagraph("box.source", DefaultSourcePrefix + TaggedTemplateBinding.Tag("slot:box.source")));

        var figureNumber = new XElement("AUTONUM", new XAttribute("Number", 1), new XAttribute("NumberType", "Figure"),
            new XElement("AUTONUMFORMAT", new XAttribute("Superscript", "false"), new XAttribute("Type", "Digit")));
        var figureCaption = Paragraph("figure.caption", new XElement("CHAR", "[그림 " + TemplateHeadingNumbers.Tag + "-"), figureNumber,
            new XElement("CHAR", "] " + TaggedTemplateBinding.Tag("slot:figure.caption")));
        var picture = new XElement(result.Descendants("PICTURE").Single());
        var pictureShape = picture.Element("SHAPEOBJECT")!;
        pictureShape.Element("POSITION")!.SetAttributeValue("TreatAsChar", "true");
        var nativeCaption = pictureShape.Element("CAPTION") ?? throw new InvalidDataException("Sample picture has no native caption.");
        nativeCaption.Element("PARALIST")!.ReplaceNodes(figureCaption,
            TextParagraph("figure.source", DefaultSourcePrefix + TaggedTemplateBinding.Tag("slot:figure.source")));
        // Retain only the empty first paragraph's native section/page definition.
        var first = new XElement(blankRoot);
        first.SetAttributeValue("Style", ids["body"]);
        var roots = new List<XElement> { first, Declaration("begin:template"), Declaration("ir-version:" + IrContract.Version) };
        var figureWidth = Math.Min(142, width * 25.4 / 7200).ToString("0.###", CultureInfo.InvariantCulture);
        foreach (var role in roles.Where(r => r == "body" || r == "footnote" || r.StartsWith("heading", StringComparison.Ordinal) || r == "reset"))
            roots.Add(Declaration(role, role));
        roots.Add(TextParagraph("body", "헤딩을 여러 문단으로 구성하려면 해당 headingN 선언을 begin:headingN … slot:headingN … end:headingN 범위로 바꾸세요. 각 이름을 {{ 및 md2hwp: 및 }}로 감싸고, 경계와 제목 슬롯은 각각 독립 문단에 둡니다. N은 1~6이며 같은 수준에 두 방식을 함께 쓰지 않습니다. 블록에 표·글상자·묶음 도형·쪽 나눔과 머리말·감추기·새 번호 제어를 함께 둘 수 있습니다. 제목 슬롯은 표 셀·글상자·머리말·꼬리말 안의 독립 문단에도 놓을 수 있고, 여러 개 두면 같은 제목으로 채웁니다. 절 목록은 begin:each.child:heading2 … slot:heading2 … end:each.child:heading2 범위로 반복하고, 안에 heading3 반복을 중첩할 수 있습니다. 같은 문단 안의 begin:once … end:once 범위는 해당 수준의 첫 헤딩에서만 포함하며, 중첩하거나 제목 슬롯을 감싸지 않습니다."));
        roots.AddRange(new[] { Declaration("list.max-depth:6"),
            Declaration("list.indent-hwp:1000") });
        foreach (var kind in new[] { "bullet", "ordered" })
        {
            var native = AuriMinimalBoxPrototype.RootParagraphs(seed).Single(p =>
                TaggedTemplateBinding.DirectText(p) == TaggedTemplateBinding.Tag("list." + kind));
            var sample = Declaration("list." + kind);
            sample.SetAttributeValue("ParaShape", (string)native.Attribute("ParaShape")!);
            roots.Add(sample);
        }
        roots.AddRange(new[] { Declaration("begin:block.box"), Paragraph("body", table), Declaration("end:block.box"),
            Declaration("figure.max-width-mm:" + figureWidth), Declaration("begin:figure.caption"),
            Paragraph("body", picture), Declaration("end:figure.caption"),
            Declaration("begin:ref.figure.number"),
            TextParagraph("ref.figure.number", "그림 " + TemplateHeadingNumbers.Tag + "-" + TemplateCrossReferences.FigureNumberSlot),
            Declaration("end:ref.figure.number"),
            Declaration("begin:ref.heading.number"),
            TextParagraph("ref.heading.number", TemplateCrossReferences.HeadingNumberSlot),
            Declaration("end:ref.heading.number"),
            Declaration("end:template"), Declaration("content"), TextParagraph("body", "") });
        section.ReplaceNodes(roots);
        return result;
    }
}
