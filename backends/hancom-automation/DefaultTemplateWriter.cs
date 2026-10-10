using System.Globalization;
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal sealed record TemplateCreationResult(string Output, string IrVersion, bool Reopened);

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

                Run(hwp, "InsertFootnote");
                InsertText(hwp, TaggedTemplateBinding.Tag("footnote"));
                Run(hwp, "Close");
                Run(hwp, "BreakPara");

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
                var xml = "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>" + TemplateObjectSources.Lower(authored).Document.ToString(SaveOptions.DisableFormatting);
                // Unlike SaveAs, SetTextFile returns an integer status (1 = success).
                object imported = hwp.SetTextFile(xml, "HWPML2X", "");
                if (imported is not int { } status || status != 1)
                    throw new InvalidOperationException($"Could not import the default template declarations: {imported} ({imported?.GetType().Name}).");
                // Locate each native caption's slot. Hancom MoveLeft leaves the
                // selection one character before its start; MoveRight returns to
                // the tag boundary without depending on the prefix length.
                foreach (var role in new[] { "code.source", "figure.source", "table.source" })
                {
                    Run(hwp, "MoveDocBegin");
                    FindTaggedText(hwp, TaggedTemplateBinding.Tag("slot:" + role));
                    Run(hwp, "MoveLeft");
                    Run(hwp, "MoveRight");
                    Run(hwp, "ParagraphShapeIndentAtCaret");
                }
                XDocument adjusted = HwpMarkup.Parse((string)hwp.GetTextFile("HWPML2X", ""));
                foreach (var role in new[] { "code.source", "figure.source", "table.source" })
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
                foreach (var name in new[] { "md2hwp.code.source", "md2hwp.figure.source", "md2hwp.table.source" })
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
        var roles = new[] { "body", "heading1", "heading2", "heading3", "heading4", "heading5", "heading6", "footnote", "code", "code.title", "code.source", "figure.caption", "figure.source", "table.header", "table.content", "table.source", "table.caption", "ref.figure.number", "ref.table.number", "reset" };
        roles = roles.Append("footnote.next").Concat(Enumerable.Range(1, 6).Select(TemplateCrossReferences.HeadingRole)).ToArray();
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
        cell.Elements().Single(e => e.Name.LocalName == "PARALIST").ReplaceNodes(Declaration("slot:code.title", "code.title"), Declaration("slot:code.content", "code"));
        var caption = shape.Elements().Single(e => e.Name.LocalName == "CAPTION");
        caption.SetAttributeValue("LastWidth", width);
        caption.Elements().Single(e => e.Name.LocalName == "PARALIST").ReplaceNodes(TextParagraph("code.source", TemplateObjectSources.PrefixSlot("code") + ": " + TaggedTemplateBinding.Tag("slot:code.source")));

        var figureNumber = new XElement("AUTONUM", new XAttribute("Number", 1), new XAttribute("NumberType", "Figure"),
            new XElement("AUTONUMFORMAT", new XAttribute("Superscript", "false"), new XAttribute("Type", "Digit")));
        var figureCaption = Paragraph("figure.caption", new XElement("CHAR", "[그림 " + TemplateHeadingNumbers.Tag + "-"), figureNumber,
            new XElement("CHAR", "] " + TaggedTemplateBinding.Tag("slot:figure.caption")));
        var picture = new XElement(result.Descendants("PICTURE").Single());
        var pictureShape = picture.Element("SHAPEOBJECT")!;
        pictureShape.Element("POSITION")!.SetAttributeValue("TreatAsChar", "true");
        var nativeCaption = pictureShape.Element("CAPTION") ?? throw new InvalidDataException("Sample picture has no native caption.");
        nativeCaption.Element("PARALIST")!.ReplaceNodes(figureCaption,
            TextParagraph("figure.source", TemplateObjectSources.PrefixSlot("figure") + ": " + TaggedTemplateBinding.Tag("slot:figure.source")));
        // Two header/body cells own outer and internal vertical borders.
        // Their left cells own all other formatting; the source spans both columns.
        var tableSample = new XElement(seed.Descendants("TABLE").Single());
        foreach (var identity in tableSample.DescendantsAndSelf().Attributes().Where(a => a.Name.LocalName is "InstId" or "InstID").ToArray())
            identity.Remove();
        tableSample.SetAttributeValue("RowCount", 3);
        tableSample.SetAttributeValue("ColCount", 2);
        tableSample.SetAttributeValue("PageBreak", "Cell");
        tableSample.SetAttributeValue("RepeatHeader", "true");
        var tableShape = tableSample.Element("SHAPEOBJECT")!;
        tableShape.SetAttributeValue("NumberingType", "Table");
        tableShape.Element("POSITION")!.SetAttributeValue("TreatAsChar", "false");
        tableShape.Element("SIZE")!.SetAttributeValue("Width", width);
        var tableCaption = tableShape.Element("CAPTION") ?? throw new InvalidDataException("Default table has no native caption.");
        tableCaption.SetAttributeValue("Side", "Top");
        tableCaption.SetAttributeValue("LastWidth", width);
        var tableNumber = new XElement("AUTONUM", new XAttribute("Number", 1), new XAttribute("NumberType", "Table"),
            new XElement("AUTONUMFORMAT", new XAttribute("Superscript", "false"), new XAttribute("Type", "Digit")));
        tableCaption.Element("PARALIST")!.ReplaceNodes(Paragraph("table.caption",
            new XElement("CHAR", "[표 " + TemplateHeadingNumbers.Tag + "-"), tableNumber,
            new XElement("CHAR", "] " + TaggedTemplateBinding.Tag("slot:table.caption"))));
        var sampleCell = new XElement(tableSample.Elements("ROW").Single().Elements("CELL").Single());
        var sourceBorder = new XElement(result.Descendants("BORDERFILL").Single(e =>
            (string?)e.Attribute("Id") == (string?)sampleCell.Attribute("BorderFill")));
        var borderList = result.Descendants("BORDERFILL").First().Parent!;
        sourceBorder.SetAttributeValue("Id", borderList.Elements("BORDERFILL").Max(e => (int)e.Attribute("Id")!) + 1);
        sourceBorder.Elements("FILLBRUSH").Remove();
        foreach (var border in sourceBorder.Elements().Where(e => e.Name.LocalName.EndsWith("BORDER", StringComparison.Ordinal) || e.Name.LocalName == "DIAGONAL"))
            border.SetAttributeValue("Type", "None");
        var contentBottom = result.Descendants("BORDERFILL").Single(e =>
            (string?)e.Attribute("Id") == (string?)sampleCell.Attribute("BorderFill")).Element("BOTTOMBORDER")!;
        sourceBorder.Element("TOPBORDER")!.ReplaceWith(new XElement("TOPBORDER", contentBottom.Attributes(), contentBottom.Nodes()));
        foreach (var name in new[] { "Slash", "BackSlash", "CounterSlash", "CounterBackSlash", "CrookedSlash", "CrookedBackSlash", "CenterLine" })
            if (sourceBorder.Attribute(name) is not null) sourceBorder.SetAttributeValue(name, 0);
        sourceBorder.SetAttributeValue("Shadow", "false"); sourceBorder.SetAttributeValue("ThreeD", "false");
        borderList.Add(sourceBorder); borderList.SetAttributeValue("Count", borderList.Elements().Count());
        tableSample.Elements("ROW").Remove();
        foreach (var (role, rowIndex) in new[] { ("table.header", 0), ("table.content", 1), ("table.source", 2) })
        {
            var row = new XElement("ROW");
            for (var column = 0; column < (rowIndex == 2 ? 1 : 2); column++)
            {
                var sampleRowCell = new XElement(sampleCell);
                sampleRowCell.SetAttributeValue("Width", rowIndex == 2 ? width : column == 0 ? width / 2 : width - width / 2);
                sampleRowCell.SetAttributeValue("RowAddr", rowIndex);
                sampleRowCell.SetAttributeValue("ColAddr", column);
                sampleRowCell.SetAttributeValue("RowSpan", 1);
                sampleRowCell.SetAttributeValue("ColSpan", rowIndex == 2 ? 2 : 1);
                sampleRowCell.SetAttributeValue("Header", rowIndex == 0 ? "true" : "false");
                if (rowIndex == 2) sampleRowCell.SetAttributeValue("BorderFill", (string)sourceBorder.Attribute("Id")!);
                sampleRowCell.Element("PARALIST")!.ReplaceNodes(TextParagraph(role,
                    (rowIndex == 2 ? TemplateObjectSources.PrefixSlot("table") + ": " : "") + TaggedTemplateBinding.Tag("slot:" + role)));
                row.Add(sampleRowCell);
            }
            tableSample.Add(row);
        }
        // Retain only the empty first paragraph's native section/page definition.
        var first = new XElement(blankRoot);
        first.SetAttributeValue("Style", ids["body"]);
        var roots = new List<XElement> { first, Declaration("begin:template"), Declaration("ir-version:" + IrContract.Version) };
        var figureWidth = Math.Min(142, width * 25.4 / 7200).ToString("0.###", CultureInfo.InvariantCulture);
        foreach (var role in roles.Where(r => r == "body" || r.StartsWith("heading", StringComparison.Ordinal) || r == "reset"))
            roots.Add(Declaration(role, role));
        var footnote = new XElement(seed.Descendants("FOOTNOTE").Single());
        var footnoteBody = TextParagraph("footnote", " " + TaggedTemplateBinding.Tag("footnote"));
        footnoteBody.Element("TEXT")!.AddFirst(new XElement(footnote.Descendants("AUTONUM").Single()));
        footnote.Element("PARALIST")!.ReplaceNodes(footnoteBody,
            TextParagraph("footnote.next", TaggedTemplateBinding.Tag("footnote.next")));
        roots.Add(Paragraph("body", new XElement("CHAR", "각주 서식 샘플"), footnote));
        roots.Add(TextParagraph("body", "헤딩을 여러 문단으로 구성하려면 해당 headingN 선언을 begin:headingN … slot:headingN … end:headingN 범위로 바꾸세요. 각 이름을 {{ 및 md2hwp: 및 }}로 감싸고, 경계와 제목 슬롯은 각각 독립 문단에 둡니다. N은 1~6이며 같은 수준에 두 방식을 함께 쓰지 않습니다. 블록에 표·글상자·묶음 도형·쪽 나눔과 머리말·감추기·새 번호 제어를 함께 둘 수 있습니다. 제목 슬롯은 표 셀·글상자·머리말·꼬리말 안의 독립 문단에도 놓을 수 있고, 여러 개 두면 같은 제목으로 채웁니다. 절 목록은 begin:each.child:heading2 … slot:heading2 … end:each.child:heading2 범위로 반복하고, 안에 heading3 반복을 중첩할 수 있습니다. begin:once … end:once 범위는 해당 수준의 첫 헤딩에만, begin:except.once … end:except.once 범위는 두 번째 이후 헤딩에만 적용합니다. 같은 컨테이너의 문단 중간에도 태그를 둘 수 있고, 서로 다른 문단에 두면 사이의 내용과 문단 경계 및 끝 태그 문단의 쪽 나눔을 함께 처리합니다. 같은 문단의 쪽 나눔 속성은 바꾸지 않습니다. 중첩하거나 제목 슬롯을 범위 안에 넣지 않습니다."));
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
        roots.AddRange(new[] { Declaration("begin:code"), Paragraph("body", table), Declaration("end:code"),
            Declaration("figure.max-width-mm:" + figureWidth), Declaration("begin:figure"),
            Paragraph("body", picture), Declaration("end:figure"),
            Declaration("table.width-mm:" + figureWidth), Declaration("begin:table"),
            Paragraph("body", tableSample), Declaration("end:table"),
            Declaration("begin:ref.figure.number"),
            TextParagraph("ref.figure.number", "그림 " + TemplateHeadingNumbers.Tag + "-" + TemplateCrossReferences.FigureNumberSlot),
            Declaration("end:ref.figure.number"),
            Declaration("begin:ref.table.number"),
            TextParagraph("ref.table.number", "표 " + TemplateHeadingNumbers.Tag + "-" + TemplateCrossReferences.TableNumberSlot),
            Declaration("end:ref.table.number"),
            Declaration("end:template"), Declaration("content"), TextParagraph("body", "") });
        var referenceBlocks = new List<XElement>();
        for (var level = 1; level <= 6; level++)
        {
            var role = TemplateCrossReferences.HeadingRole(level);
            var number = string.Join(".", Enumerable.Range(1, level).Select(index => TaggedTemplateBinding.Tag($"num:heading{index}")));
            var text = level == 1 ? "제" + number + "장" : number + (level == 2 ? "절" : "항");
            referenceBlocks.AddRange([Declaration("begin:" + role), TextParagraph(role, text), Declaration("end:" + role)]);
        }
        roots.InsertRange(roots.FindIndex(root => TaggedTemplateBinding.DirectText(root) == TaggedTemplateBinding.Tag("end:template")), referenceBlocks);
        section.ReplaceNodes(roots);
        return result;
    }
}
