using System.Globalization;
using System.Text.RegularExpressions;
using System.Xml.Linq;

namespace Md2Hwp.HancomIrPreview;

// The native table break/repeated-header options do not keep the first header
// with a body row. Read the actual cell positions after import, then move only
// an orphaned generated table start. Never make a long table indivisible.
internal static class NativeTablePagination
{
    internal sealed record Anchor(int RootParagraph, int Columns);
    internal sealed record StartPages(int Header, int FirstBody, int HeaderColumn = 1, int FirstBodyColumn = 1)
    { internal bool Together => Header == FirstBody && HeaderColumn == FirstBodyColumn; }
    private static readonly Regex CellAddress = new(@"\(([A-Z]+)([1-9][0-9]*)\)", RegexOptions.CultureInvariant);

    // Resolve generated identities before import: Hancom can assign new drawing
    // identities. The caller verifies the whole imported structure before using
    // these private adapter coordinates.
    internal static IReadOnlyList<Anchor> ResolveAnchors(XDocument document, IReadOnlyCollection<string> bodyInstances)
    {
        var generated = bodyInstances.ToHashSet(StringComparer.Ordinal);
        var roots = AuriMinimalBoxPrototype.RootParagraphs(document);
        var matches = roots.Select((paragraph, index) => (paragraph, index, tables: paragraph.Descendants("TABLE")
            .Where(table => generated.Contains((string?)table.Element("SHAPEOBJECT")?.Attribute("InstId") ??
                (string?)table.Element("SHAPEOBJECT")?.Attribute("InstID") ?? "")).ToArray()))
            .Where(item => item.tables.Length > 0).ToArray();
        if (matches.Sum(item => item.tables.Length) != generated.Count || matches.Any(item => item.tables.Length != 1))
            throw new InvalidOperationException("Generated table starts cannot be resolved to distinct root paragraphs.");
        return matches.Select(item => new Anchor(item.index, (int?)item.tables[0].Attribute("ColCount") ??
            throw new InvalidOperationException("Generated table has no native column count."))).ToArray();
    }

    internal static int Reflow(dynamic hwp, XDocument document, IReadOnlyCollection<Anchor> anchors,
        Action<XDocument> reimport) => Correct(document, anchors, anchor => ReadStartPages(hwp, anchor), reimport);

    internal static int Correct(XDocument document, IReadOnlyCollection<Anchor> anchors,
        Func<Anchor, StartPages> readPages, Action<XDocument> reimport)
    {
        var roots = AuriMinimalBoxPrototype.RootParagraphs(document);
        if (anchors.Select(anchor => anchor.RootParagraph).Distinct().Count() != anchors.Count)
            throw new InvalidOperationException("Duplicate generated table pagination anchor.");
        var corrected = 0;
        foreach (var anchor in anchors.OrderBy(anchor => anchor.RootParagraph))
        {
            if (anchor.RootParagraph < 0 || anchor.RootParagraph >= roots.Count)
                throw new InvalidOperationException("Missing generated table pagination root.");
            var paragraph = roots[anchor.RootParagraph];
            var tables = paragraph.Descendants("TABLE").ToArray();
            if (tables.Length != 1 || anchor.Columns <= 0 || (int?)tables[0].Attribute("ColCount") != anchor.Columns ||
                tables[0].Elements("ROW").Count() < 2)
                throw new InvalidOperationException("Generated table pagination requires a matching header and first body row.");
            var pages = RequirePages(readPages(anchor));
            if (pages.Together) continue;
            // A second forced break cannot help when the table already starts a
            // fresh page. Preserve the old successful output instead of looping.
            if ((string?)paragraph.Attribute("PageBreak") == "true")
                throw CannotFit(anchor);
            paragraph.SetAttributeValue("PageBreak", "true");
            paragraph.SetAttributeValue("ColumnBreak", "false");
            reimport(document);
            if (RequirePages(readPages(anchor)) is { } after && !after.Together)
                throw CannotFit(anchor);
            corrected++;
            // Re-read later tables after each native reflow. Earlier corrections
            // may already have removed a later table's orphaned header.
        }
        return corrected;
    }

    internal static void VerifyReopened(dynamic hwp, IReadOnlyCollection<Anchor> anchors) =>
        Verify(anchors, anchor => ReadStartPages(hwp, anchor));

    internal static void Verify(IReadOnlyCollection<Anchor> anchors, Func<Anchor, StartPages> readPages)
    {
        foreach (var anchor in anchors)
            if (!RequirePages(readPages(anchor)).Together) throw CannotFit(anchor);
    }

    private static StartPages RequirePages(StartPages pages)
    {
        if (pages.Header <= 0 || pages.FirstBody <= 0 || pages.FirstBody < pages.Header ||
            pages.HeaderColumn <= 0 || pages.FirstBodyColumn <= 0 ||
            pages.Header == pages.FirstBody && pages.FirstBodyColumn < pages.HeaderColumn)
            throw new InvalidOperationException("Native table cell page positions are missing or inconsistent.");
        return pages;
    }

    private static InvalidOperationException CannotFit(Anchor anchor) => new(
        $"The table header and first body row cannot start on the same page/column at root paragraph {anchor.RootParagraph}. " +
        "Reduce the header/caption size or the first row's formatting in the template.");

    private static StartPages ReadStartPages(dynamic hwp, Anchor anchor)
    {
        dynamic? table = null;
        for (dynamic control = hwp.HeadCtrl; control is not null; control = control.Next)
        {
            if ((string)control.CtrlID != "tbl") continue;
            dynamic candidate = control.GetAnchorPos(0);
            if (Integer(candidate.Item("List")) != 0 || Integer(candidate.Item("Para")) != anchor.RootParagraph) continue;
            if (table is not null) throw new InvalidOperationException("Ambiguous generated table native anchor.");
            table = control;
        }
        if (table is null) throw new InvalidOperationException("Missing generated table native anchor.");
        if (!(bool)hwp.SetPosBySet(table.GetAnchorPos(0)))
            throw new InvalidOperationException("Could not move to the generated table anchor.");
        if ((string)hwp.FindCtrl() != "tbl")
            throw new InvalidOperationException("Could not select the generated table control.");
        Run(hwp, "ShapeObjTableSelCell"); Run(hwp, "Cancel"); Run(hwp, "MoveListBegin");
        int list, paragraph, position;
        hwp.GetPos(out list, out paragraph, out position);
        if (list <= 0 || paragraph != 0 || position != 0)
            throw new InvalidOperationException("Could not enter the first generated table header cell.");
        var header = ReadRow((object)hwp, anchor, 1);
        if (!(bool)hwp.SetPos(list, 0, 0))
            throw new InvalidOperationException("Could not return to the first generated table header cell.");
        Run(hwp, "TableLowerCell"); Run(hwp, "MoveListBegin");
        var body = ReadRow((object)hwp, anchor, 2);
        return new(header.Page, body.Page, header.Column, body.Column);
    }

    private static (int Page, int Column) ReadRow(object automation, Anchor anchor, int row)
    {
        dynamic hwp = automation;
        var first = (Page: int.MaxValue, Column: int.MaxValue);
        for (var column = 0; column < anchor.Columns; column++)
        {
            dynamic parent = hwp.ParentCtrl;
            if (parent is null || (string)parent.CtrlID != "tbl")
                throw new InvalidOperationException("Native table traversal left the generated table.");
            dynamic parentAnchor = parent.GetAnchorPos(0);
            if (Integer(parentAnchor.Item("List")) != 0 || Integer(parentAnchor.Item("Para")) != anchor.RootParagraph)
                throw new InvalidOperationException("Native table traversal reached another table.");
            int sections, section, page, currentColumn, line, position;
            short overwrite;
            string name;
            if (!(bool)hwp.KeyIndicator(out sections, out section, out page, out currentColumn, out line,
                    out position, out overwrite, out name) || page <= 0 || currentColumn <= 0)
                throw new InvalidOperationException("Could not read the native table cell page.");
            var match = CellAddress.Match(name);
            if (!match.Success || match.Groups[1].Value != ColumnName(column) ||
                match.Groups[2].Value != row.ToString(CultureInfo.InvariantCulture))
                throw new InvalidOperationException("Native table traversal did not reach the expected cell address.");
            if (page < first.Page || page == first.Page && currentColumn < first.Column) first = (page, currentColumn);
            if (column + 1 < anchor.Columns) { Run(hwp, "TableRightCell"); Run(hwp, "MoveListBegin"); }
        }
        return first;
    }

    private static int Integer(object value) => Convert.ToInt32(value, CultureInfo.InvariantCulture);
    private static void Run(dynamic hwp, string action)
    { if (!(bool)hwp.HAction.Run(action)) throw new InvalidOperationException($"Native table pagination failed: {action}."); }
    private static string ColumnName(int index)
    {
        var result = "";
        for (var number = index + 1; number > 0; number = (number - 1) / 26)
            result = (char)('A' + (number - 1) % 26) + result;
        return result;
    }
}
