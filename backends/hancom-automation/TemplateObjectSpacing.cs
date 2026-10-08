using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// Root spacing belongs to the object prototype, outside its native caption.
internal sealed class TemplateObjectSpacing(XDocument source, XElement[] before, XElement[] after)
{
    internal static (XElement Root, TemplateObjectSpacing Spacing) CaptureRange(
        XDocument source, XElement[] paragraphs, string objectName, string role)
    {
        var objects = paragraphs.Where(paragraph => paragraph.Descendants(objectName).Any()).ToArray();
        if (objects.Length != 1)
            throw new InvalidDataException($"{role} requires one sample object, with only empty plain paragraphs before and after it.");
        var index = Array.IndexOf(paragraphs, objects[0]);
        return (objects[0], Capture(source, paragraphs.Take(index), paragraphs.Skip(index + 1), role));
    }

    internal static TemplateObjectSpacing Capture(XDocument source, IEnumerable<XElement> paragraphs, string role) =>
        Capture(source, [], paragraphs, role);

    private static TemplateObjectSpacing Capture(XDocument source,
        IEnumerable<XElement> preceding, IEnumerable<XElement> following, string role)
    {
        var before = preceding.ToArray();
        var after = following.ToArray();
        foreach (var paragraph in before.Concat(after))
        {
            var shape = source.Descendants("PARASHAPE").SingleOrDefault(element =>
                (string?)element.Attribute("Id") == (string?)paragraph.Attribute("ParaShape"));
            if (paragraph.Elements().Any(element => element.Name.LocalName != "TEXT") ||
                paragraph.Elements("TEXT").SelectMany(text => text.Elements()).Any(element =>
                    element.Name.LocalName != "CHAR" || element.HasElements || !string.IsNullOrWhiteSpace(element.Value)) ||
                (string?)shape?.Attribute("HeadingType") is not (null or "None"))
                throw new InvalidDataException($"{role} allows only empty plain paragraphs before and after its single sample object; text, controls and list/outline numbering are unsupported.");
        }
        return new(new XDocument(source), before.Select(paragraph => new XElement(paragraph)).ToArray(),
            after.Select(paragraph => new XElement(paragraph)).ToArray());
    }

    // Run after every operation-indexed attachment, before recording layout paths.
    internal void Attach(XDocument document, string objectName, IReadOnlyCollection<string> instances)
    {
        if (before.Length == 0 && after.Length == 0) return;
        var roots = AuriMinimalBoxPrototype.RootParagraphs(document);
        var anchors = roots.SelectMany(paragraph => paragraph.Descendants(objectName).Elements("SHAPEOBJECT")
            .Select(shape => (Paragraph: paragraph, Id: (string?)shape.Attribute("InstId") ?? (string?)shape.Attribute("InstID"))))
            .Where(item => item.Id is not null).ToLookup(item => item.Id!, item => item.Paragraph, StringComparer.Ordinal);
        foreach (var instance in instances)
        {
            var matches = anchors[instance].ToArray();
            if (matches.Length != 1)
                throw new InvalidOperationException($"Generated {objectName} spacing requires one native anchor for instance {instance}.");
            foreach (var paragraph in before)
                matches[0].AddBeforeSelf(TemplateHeadingBlocks.ImportParagraph(paragraph, source, document));
            var previous = matches[0];
            foreach (var paragraph in after)
            {
                var copy = TemplateHeadingBlocks.ImportParagraph(paragraph, source, document);
                previous.AddAfterSelf(copy);
                previous = copy;
            }
        }
    }
}
