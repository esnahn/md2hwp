using System.Globalization;
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

internal sealed record TaggedTemplateBinding(InvestigationTemplateProfile Profile,
    int TemplateBegin, int TemplateEnd, int Content, int BoxRoot, int CaptionRoot)
{
    internal const string Prefix = "{{md2hwp:";
    internal static string Tag(string name) => Prefix + name + "}}";
    internal static string DirectText(XElement paragraph) => string.Concat(
        paragraph.Elements().Where(e => e.Name.LocalName == "TEXT")
            .SelectMany(e => e.Elements()).Where(e => e.Name.LocalName == "CHAR").Select(e => e.Value));

    public static TaggedTemplateBinding Read(XDocument document, string templatePath)
    {
        var prepared = TemplateHeadingNumbers.Prepare(PreserveBeginSectionSettings(document));
        prepared = NativeFootnotes.Bind(prepared).LowerSample(prepared);
        var sources = TemplateObjectSources.Lower(prepared);
        var references = TemplateCrossReferences.Lower(sources.Document);
        var tables = TemplateTables.Lower(references.Document);
        var headings = TemplateHeadingBlocks.Lower(tables.Document);
        var boxes = TemplateBoxParagraphs.Lower(headings.Document);
        return ReadFlat(NativeFigureCaption.Lower(boxes.Document).Document, templatePath);
    }

    // Section settings may own master-page paragraphs. Keep them outside the
    // disposable definitions only for begin:template; never edit the source file.
    internal static XDocument PreserveBeginSectionSettings(XDocument source)
    {
        var document = new XDocument(source);
        var roots = AuriMinimalBoxPrototype.RootParagraphs(document);
        var matches = roots.Where(p => DirectText(p) == Tag("begin:template")).ToArray();
        if (matches.Length != 1) return document;
        var begin = matches[0];
        if (begin.Elements().Any(e => e.Name.LocalName != "TEXT")) return document;
        var children = begin.Elements().SelectMany(e => e.Elements()).ToArray();
        if (!children.Any(e => e.Name.LocalName == "SECDEF") ||
            children.Any(e => e.Name.LocalName is not ("CHAR" or "SECDEF" or "COLDEF")) ||
            children.Where(e => e.Name.LocalName == "CHAR").Any(e => e.HasElements))
            return document;
        var retained = new XElement(begin);
        foreach (var text in retained.Elements().ToArray())
        {
            text.Elements().Where(e => e.Name.LocalName == "CHAR").Remove();
            if (!text.HasElements) text.Remove();
        }
        var firstText = retained.Elements().First();
        firstText.Add(new XElement("CHAR", ""));
        var declaration = new XElement(begin);
        declaration.Elements().SelectMany(e => e.Elements())
            .Where(e => e.Name.LocalName is "SECDEF" or "COLDEF").Remove();
        declaration.Elements().Where(e => !e.HasElements).Remove();
        // Hancom omits explicit false break flags on the new noninitial paragraph.
        declaration.Attributes().Where(a => (a.Name.LocalName is "PageBreak" or "ColumnBreak") && a.Value == "false").Remove();
        begin.ReplaceWith(retained, declaration);
        return document;
    }

    internal static TaggedTemplateBinding ReadFlat(XDocument document, string templatePath)
    {
        var roots = AuriMinimalBoxPrototype.RootParagraphs(document).ToList();
        var accepted = new HashSet<XElement>();
        int Single(string token)
        {
            var matches = roots.Select((p, i) => (p, i))
                .Where(x => DirectText(x.p) == Tag(token) &&
                    !x.p.Descendants().Any(e => e.Name.LocalName is "P" or "TABLE" or "PICTURE" or "AUTONUM"))
                .ToArray();
            if (matches.Length != 1) throw new InvalidDataException($"Expected one root declaration {token}; found {matches.Length}. Regenerate the template with init-template or add the required declaration.");
            accepted.Add(matches[0].p);
            return matches[0].i;
        }
        if (!roots.Any(p => DirectText(p).StartsWith(Prefix + "ir-version:", StringComparison.Ordinal)))
            throw new InvalidDataException($"Template has no IR version. Create an IR {IrContract.Version} template with init-template.");
        var begin = Single("begin:template");
        var end = Single("end:template");
        var content = Single("content");
        if (begin <= 0 || end <= begin || end + 1 != content || content != roots.Count - 2 ||
            DirectText(roots[^1]) != "" || roots[^1].Descendants().Any(e => e.Name.LocalName is "P" or "TABLE" or "PICTURE"))
            throw new InvalidDataException("Template definitions must precede the terminal content target and its empty successor.");
        void Inside(int index)
        {
            if (index <= begin || index >= end) throw new InvalidDataException($"Declaration at root[{index}] is outside template definitions.");
        }
        string Setting(string key)
        {
            var prefix = Prefix + key + ":";
            var matches = roots.Select((p, i) => (p, i, text: DirectText(p)))
                .Where(x => x.text.StartsWith(prefix, StringComparison.Ordinal) && x.text.EndsWith("}}", StringComparison.Ordinal))
                .ToArray();
            if (matches.Length != 1) throw new InvalidDataException($"Expected one setting {key}.");
            var match = matches[0];
            Inside(match.i);
            if (match.p.Descendants().Any(e => e.Name.LocalName is "P" or "TABLE" or "PICTURE" or "AUTONUM"))
                throw new InvalidDataException($"Setting {key} must be a plain root paragraph.");
            accepted.Add(match.p);
            var value = match.text[prefix.Length..^2];
            if (value.Length == 0 || value.Length > 64 || value.IndexOfAny(['{', '}', '\r', '\n']) >= 0)
                throw new InvalidDataException($"Invalid setting {key}.");
            return value;
        }
        IrContract.RequireCurrent(Setting("ir-version"), "Template");
        var widthText = Setting("figure.max-width-mm");
        if (!double.TryParse(widthText, NumberStyles.AllowDecimalPoint, CultureInfo.InvariantCulture, out var width) ||
            !double.IsFinite(width) || width <= 0 || width > 142)
            throw new InvalidDataException("Figure width must be greater than zero and at most 142 mm.");
        int IntegerSetting(string key, int maximum)
        {
            if (!int.TryParse(Setting(key), NumberStyles.None, CultureInfo.InvariantCulture, out var value) || value < 1 || value > maximum)
                throw new InvalidDataException($"Invalid positive setting {key}.");
            return value;
        }
        var depth = IntegerSetting("list.max-depth", 6);
        var indent = IntegerSetting("list.indent-hwp", 10000);
        var bulletRoot = Single("list.bullet");
        var orderedRoot = Single("list.ordered");
        Inside(bulletRoot); Inside(orderedRoot);
        var bullet = TemplateListPrototype.Read(document, roots[bulletRoot], bulletRoot, "bullet", depth);
        var ordered = TemplateListPrototype.Read(document, roots[orderedRoot], orderedRoot, "ordered", depth);
        var definitions = document.Descendants().Where(e => e.Name.LocalName == "STYLE").ToArray();
        string StyleName(XElement paragraph)
        {
            var id = (string?)paragraph.Attribute("Style");
            var matches = definitions.Where(e => (string?)e.Attribute("Id") == id && (string?)e.Attribute("Type") == "Para").ToArray();
            if (matches.Length != 1 || string.IsNullOrWhiteSpace((string?)matches[0].Attribute("Name")))
                throw new InvalidDataException("Tagged paragraph has no unique native paragraph style.");
            return matches[0].Attribute("Name")!.Value;
        }
        var styles = new List<ProfileStyle>();
        foreach (var role in new[] { "body", "heading1", "heading2", "heading3", "heading4", "heading5", "heading6", "footnote" })
        {
            var index = Single(role);
            Inside(index);
            styles.Add(new(role, StyleName(roots[index])));
        }
        _ = NativeFootnotes.Bind(document);
        var resetIndex = Single("reset");
        Inside(resetIndex);
        var reset = StyleName(roots[resetIndex]);
        if (reset == styles.Single(s => s.Symbolic == "body").NativeName)
            throw new InvalidDataException("Reset and body styles must differ.");

        var boxBegin = Single("begin:block.box");
        var boxEnd = Single("end:block.box");
        Inside(boxBegin); Inside(boxEnd);
        if (boxEnd != boxBegin + 2) throw new InvalidDataException("Box range must contain exactly one root paragraph.");
        var box = roots[boxBegin + 1];
        var tables = box.Descendants().Where(e => e.Name.LocalName == "TABLE").ToArray();
        if (tables.Length != 1)
            throw new InvalidDataException("Box must contain exactly one table.");
        styles.Add(new("box.anchor", StyleName(box)));
        var boxSlots = tables[0].Descendants().Where(e => e.Name.LocalName == "P" && DirectText(e) == Tag("slot:box.content")).ToArray();
        if (boxSlots.Length != 1 || !boxSlots[0].Ancestors().Any(e => e.Name.LocalName == "CELL"))
            throw new InvalidDataException("Box requires one content slot inside its cell.");
        accepted.Add(boxSlots[0]);
        styles.Add(new("block.box", StyleName(boxSlots[0])));
        var boxSourceSlot = Tag("slot:box.source");
        var boxSources = tables[0].Descendants().Where(e => e.Name.LocalName == "P" &&
            DirectText(e).Contains(boxSourceSlot, StringComparison.Ordinal)).ToArray();
        if (boxSources.Length != 1) throw new InvalidDataException("Box requires exactly one source slot in its native caption.");
        var boxSource = TemplateSource.Read(boxSources[0], boxSourceSlot);
        if (!boxSources[0].Ancestors().Any(e => e.Name.LocalName == "CAPTION"))
            throw new InvalidDataException("Box source must be in its native table caption.");
        accepted.Add(boxSources[0]);
        styles.Add(new("box.source", StyleName(boxSources[0])));

        var figureBegin = Single("begin:figure");
        var figureEnd = Single("end:figure");
        Inside(figureBegin); Inside(figureEnd);
        if (figureEnd != figureBegin + 4 || !(boxEnd < figureBegin || figureEnd < boxBegin))
            throw new InvalidDataException("Figure range must contain image, caption, source in order without overlap.");
        var image = Single("slot:figure.image");
        var source = figureBegin + 3;
        var figureSource = TemplateSource.Read(roots[source], Tag("slot:figure.source"));
        accepted.Add(roots[source]);
        if (image != figureBegin + 1 || source != figureBegin + 3)
            throw new InvalidDataException("Figure slot is outside its exact range position.");
        var caption = roots[figureBegin + 2];
        var captionSlot = Tag("slot:figure.caption");
        var captionText = DirectText(caption);
        if (captionText.Split(captionSlot, StringSplitOptions.None).Length != 2 ||
            captionText.Replace(captionSlot, "", StringComparison.Ordinal).Contains(Prefix, StringComparison.Ordinal) ||
            caption.Descendants().Count(e => e.Name.LocalName == "AUTONUM" && (string?)e.Attribute("NumberType") == "Figure") != 1 ||
            caption.Descendants().Any(e => e.Name.LocalName is "P" or "TABLE" or "PICTURE"))
            throw new InvalidDataException("Caption must preserve its native Figure AUTONUM and exact text slot.");
        accepted.Add(caption);
        if (caption.Descendants().Count(e => e.Name.LocalName == "AUTONUM") != 1)
            throw new InvalidDataException("Caption must contain exactly one automatic numbering control.");
        styles.Add(new("figure", StyleName(roots[image])));
        styles.Add(new("figure.caption", StyleName(caption)));
        styles.Add(new("figure.source", StyleName(roots[source])));
        // Reject hidden, duplicate, malformed, or unknown declarations, including
        // tokens in headers/footers/cells other than the declared box slot.
        foreach (var paragraph in document.Descendants().Where(e => e.Name.LocalName == "P"))
            if (DirectText(paragraph).Contains(Prefix, StringComparison.Ordinal) && !accepted.Contains(paragraph))
                throw new InvalidDataException($"Unbound declaration: {DirectText(paragraph)}");
        if (accepted.Any(p => p.Descendants().Any(e => e.Name.LocalName == "LINEBREAK")))
            throw new InvalidDataException("Declarations must occupy a single logical line.");
        foreach (var index in accepted.Select(p => roots.IndexOf(p)).Where(i => i >= 0 && i != begin && i != end && i != content))
            if (index > boxBegin && index < boxEnd && index != boxBegin + 1 ||
                index > figureBegin && index < figureEnd && index != image && index != source && index != figureBegin + 2)
                throw new InvalidDataException("Declaration is nested in another role.");
        var number = caption.Descendants().Single(e => e.Name.LocalName == "AUTONUM");
        var elements = caption.Elements().SelectMany(e => e.Elements()).ToArray();
        if (caption.Elements().Any(e => e.Name.LocalName != "TEXT") ||
            elements.Any(e => e.Name.LocalName is not ("CHAR" or "AUTONUM")) ||
            elements.Where(e => e.Name.LocalName == "CHAR").Any(e => e.Nodes().Any(n => n is not XText)) ||
            !AuriMinimalCaptionPrototype.IsFigureAutoNumber(number))
            throw new InvalidDataException("Caption must be one text paragraph with exactly one native Figure AUTONUM.");
        var beforeNumber = string.Concat(elements.TakeWhile(e => e != number).Where(e => e.Name.LocalName == "CHAR").Select(e => e.Value));
        var afterNumber = string.Concat(elements.SkipWhile(e => e != number).Skip(1).Where(e => e.Name.LocalName == "CHAR").Select(e => e.Value));
        if (!beforeNumber.Contains(captionSlot, StringComparison.Ordinal) && !afterNumber.Contains(captionSlot, StringComparison.Ordinal))
            throw new InvalidDataException("Caption slot must not cross the automatic number control.");
        var selector = new ProfileCaptionSelector("figure.caption", beforeNumber, afterNumber, captionSlot);
        var profile = InvestigationTemplateProfile.FromTaggedTemplate(templatePath, styles, reset, width, depth, indent, selector, boxSource, figureSource, bullet, ordered);
        _ = AuriPreviewStyleBindings.BindDocument(document, profile);
        return new(profile, begin, end, content, boxBegin + 1, figureBegin + 2);
    }
}
