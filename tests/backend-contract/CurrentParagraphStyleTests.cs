using System.Xml;
using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class CurrentParagraphStyleTests
{
    internal static void Run()
    {
        foreach (var localId in new[] { "0", "3", "47" })
        {
            var block = Selected(localId);
            var before = new XDocument(block);
            Check(CurrentParagraphStyle.SelectedStyleName(block) == "본문", "Block-local style IDs were treated as document IDs.");
            Check(XNode.DeepEquals(block, before), "Style reading mutated its snapshot.");
        }
        var controls = Selected();
        Root(controls).Element("TEXT")!.AddFirst(new XElement("COLDEF"), new XElement("SECDEF",
            new XElement("HEADER", new XElement("PARALIST", Paragraph("nested header", "0")))));
        Check(CurrentParagraphStyle.SelectedStyleName(controls) == "본문", "Copied section controls changed the selected root identity.");
        foreach (var invalid in new string?[] { null, "", "-1", "unknown", "2147483648" })
            Broken(d => Root(d).SetAttributeValue("Style", invalid));
        foreach (var invalid in new string?[] { null, "" })
            Broken(d => d.Descendants("STYLE").Single().SetAttributeValue("Name", invalid));
        Broken(d => d.Descendants("STYLE").Remove());
        Broken(d => d.Descendants("STYLE").Single().AddAfterSelf(new XElement(d.Descendants("STYLE").Single())));
        Broken(d => d.Descendants("STYLE").Single().SetAttributeValue("Type", "Char"));
        foreach (var text in new[] { "", "xx", " x", "original text" })
            Broken(d => Root(d).Element("TEXT")!.Element("CHAR")!.Value = text);
        Broken(d => d.Descendants("SECTION").Single().RemoveNodes());
        Broken(d => Root(d).AddAfterSelf(Paragraph("", "3")));
        Broken(d => Root(d).Element("TEXT")!.Add(new XElement("TABLE")));
        Broken(d => Root(d).Add(new XElement("TABLE")));
        Broken(d => Root(d).Element("TEXT")!.Element("CHAR")!.Add(new XElement("LINEBREAK")));

        foreach (var original in new[] { "", "기존 내용" })
        foreach (var mode in new[] { 1, 0x11 })
        {
            var probe = new Fake(original) { BlockMode = mode };
            Check(CurrentParagraphStyle.Read(probe) == "본문", "Wrong probed native name.");
            Restored(probe, original);
            Check(probe.Inserts == 1 && probe.Deletes == 1 && probe.BlockReads == 1 && probe.FullReads == 0, "Unexpected probe read/edit count.");
            Check(probe.Begin == new ParagraphProbePosition(0, 7, original.Length) &&
                probe.End == new ParagraphProbePosition(0, 7, original.Length + 1), "The selection included original content.");
        }
        var tracked = new Fake("원문") { TrackingChanges = true };
        Root(tracked.Full).SetAttributeValue("Style", "0");
        tracked.Full.Descendants("STYLELIST").Single().Add(new XElement("STYLE", new XAttribute("Id", "0"), new XAttribute("Type", "Para"), new XAttribute("Name", "바탕글")));
        Root(tracked.Full).AddAfterSelf(Paragraph("", "3"));
        Check(CurrentParagraphStyle.Read(tracked) == "본문", "Tracked fallback did not use the final root style.");
        Restored(tracked, "원문");
        Check(tracked.Inserts == 0 && tracked.Deletes == 0 && tracked.BlockReads == 0 && tracked.FullReads == 1, "Tracking fallback edited the document.");
        foreach (var failure in new Exception[] { new IOException("export failed"), new XmlException("parse failed") })
        {
            var probe = new Fake("원문") { ReadFailure = failure };
            Check(ReferenceEquals(Reject<InvalidOperationException>(() => CurrentParagraphStyle.Read(probe)).InnerException, failure), "The read error was lost.");
            Restored(probe, "원문");
        }
        var malformed = new Fake("원문"); Root(malformed.Block).SetAttributeValue("Style", "missing");
        Check(Reject<InvalidOperationException>(() => CurrentParagraphStyle.Read(malformed)).InnerException is InvalidDataException, "Malformed style was silently accepted.");
        Restored(malformed, "원문");
        var insertFailure = new Fake("원문") { InsertFailure = new IOException("insert failed") };
        Reject<InvalidOperationException>(() => CurrentParagraphStyle.Read(insertFailure));
        Restored(insertFailure, "원문");
        Check(insertFailure.Deletes == 0 && insertFailure.BlockReads == 0, "Failed insertion backspaced original content.");
        foreach (var mode in new[] { 0, 2, 0x12 })
        {
            var probe = new Fake("원문") { BlockMode = mode };
            Reject<InvalidOperationException>(() => CurrentParagraphStyle.Read(probe)); Restored(probe, "원문");
            Check(probe.BlockReads == 0 && probe.Deletes == 1, "Invalid selection was exported or left a marker.");
        }
        var notSelected = new Fake("원문") { SelectResult = false };
        Reject<InvalidOperationException>(() => CurrentParagraphStyle.Read(notSelected)); Restored(notSelected, "원문");
        Check(notSelected.BlockReads == 0, "Failed SelectText was trusted.");
        foreach (var bad in new[] { new ParagraphProbePosition(1, 7, 0), new(0, -1, 0), new(0, 7, -1) })
        {
            var probe = new Fake { InitialEnd = bad };
            Reject<InvalidOperationException>(() => CurrentParagraphStyle.Read(probe));
            Check(probe.Inserts == 0 && probe.Deletes == 0, "Invalid initial coordinates caused editing.");
        }
        var wrongInsert = new Fake("원문") { AfterInsert = new(0, 7, 99) };
        Reject<InvalidOperationException>(() => CurrentParagraphStyle.Read(wrongInsert)); Restored(wrongInsert, "원문");
        Check(wrongInsert.BlockReads == 0, "Unexpected insertion coordinates were trusted.");
        foreach (var unsafeEnd in new[] { new ParagraphProbePosition(0, 7, 2), new(0, 8, 3), new(1, 7, 3) })
        {
            var probe = new Fake("원문") { CleanupEnd = unsafeEnd };
            Reject<InvalidOperationException>(() => CurrentParagraphStyle.Read(probe));
            Check(probe.Deletes == 0 && probe.Tail == "원문x", "Unsafe cleanup deleted original content or a paragraph boundary.");
        }
        var deletion = new IOException("delete failed");
        var deleteFailed = new Fake("원문") { DeleteFailure = deletion };
        Check(ReferenceEquals(Reject<InvalidOperationException>(() => CurrentParagraphStyle.Read(deleteFailed)).InnerException, deletion), "Cleanup failure was lost.");
        var primary = new XmlException("parse failed");
        var bothFailed = new Fake("원문") { ReadFailure = primary, DeleteFailure = deletion };
        var combined = Reject<InvalidOperationException>(() => CurrentParagraphStyle.Read(bothFailed));
        Check(combined.InnerException is AggregateException a && a.InnerExceptions.Count == 2 &&
            ReferenceEquals(a.InnerExceptions[0], primary) && ReferenceEquals(a.InnerExceptions[1], deletion), "Primary and cleanup errors were not both preserved.");
        var wrongRestore = new Fake("원문") { AfterDelete = new(0, 7, 0) };
        Reject<InvalidOperationException>(() => CurrentParagraphStyle.Read(wrongRestore));
        Check(wrongRestore.Tail == "원문" && wrongRestore.Deletes == 1, "Failed restoration deleted another original character.");
        var cancelFailed = new Fake("원문") { CancelFailure = new IOException("cancel failed") };
        Reject<InvalidOperationException>(() => CurrentParagraphStyle.Read(cancelFailed));
        Check(cancelFailed.Deletes == 0, "Cleanup backspaced with an active selection.");
        var stillSelected = new Fake("원문") { RetainSelection = true };
        Reject<InvalidOperationException>(() => CurrentParagraphStyle.Read(stillSelected));
        Check(stillSelected.Deletes == 0 && stillSelected.Tail == "원문x", "Cleanup deleted content while selection remained active.");
        Console.WriteLine("Current paragraph style identity, tracked-edit fallback and guarded marker cleanup contracts passed.");
    }

    private static void Broken(Action<XDocument> change) { var d = Selected(); change(d); Reject<InvalidDataException>(() => CurrentParagraphStyle.SelectedStyleName(d)); }
    private static XDocument Selected(string id = "3") => new(new XElement("HWPML",
        new XElement("HEAD", new XElement("STYLELIST", new XElement("STYLE", new XAttribute("Id", id), new XAttribute("Type", "Para"), new XAttribute("Name", "본문")))),
        new XElement("BODY", new XElement("SECTION", Paragraph(CurrentParagraphStyle.Marker, id)))));
    private static XElement Paragraph(string text, string id) => new("P", new XAttribute("Style", id), new XAttribute("ParaShape", "37"),
        new XElement("TEXT", new XAttribute("CharShape", "29"), new XElement("CHAR", text)));
    private static XElement Root(XDocument d) => d.Descendants("SECTION").Single().Elements("P").First();
    private static void Restored(Fake p, string original) => Check(p.Tail == original && p.Position() == new ParagraphProbePosition(0, 7, original.Length) && p.SelectionMode == 0, "Original terminal content/cursor/selection was not restored.");
    private static void Check(bool ok, string message) { if (!ok) throw new Exception(message); }
    private static T Reject<T>(Action action) where T : Exception { try { action(); } catch (T error) { return error; } throw new Exception($"Expected {typeof(T).Name}."); }

    private sealed class Fake(string original = "") : IParagraphStyleProbe
    {
        internal string Tail { get; private set; } = original;
        internal XDocument Block { get; } = Selected();
        internal XDocument Full { get; } = Selected();
        internal int Inserts, Deletes, BlockReads, FullReads;
        internal int BlockMode { get; init; } = 1;
        internal bool SelectResult { get; init; } = true;
        internal bool RetainSelection { get; init; }
        internal ParagraphProbePosition? InitialEnd { get; init; }
        internal ParagraphProbePosition? AfterInsert { get; init; }
        internal ParagraphProbePosition? CleanupEnd { get; init; }
        internal ParagraphProbePosition? AfterDelete { get; init; }
        internal Exception? InsertFailure { get; init; }
        internal Exception? ReadFailure { get; init; }
        internal Exception? DeleteFailure { get; init; }
        internal Exception? CancelFailure { get; init; }
        internal ParagraphProbePosition? Begin, End;
        private ParagraphProbePosition cursor = new(0, 7, original.Length);
        public bool TrackingChanges { get; init; }
        public int SelectionMode { get; private set; }
        public ParagraphProbePosition Position() => cursor;
        public void Run(string action)
        {
            switch (action)
            {
                case "Cancel":
                    if (Inserts > 0 && CancelFailure is not null) throw CancelFailure;
                    if (!RetainSelection) SelectionMode = 0; break;
                case "MoveDocEnd": cursor = Inserts == 0 ? InitialEnd ?? new(0, 7, Tail.Length) : CleanupEnd ?? new(0, 7, Tail.Length); break;
                case "DeleteBack":
                    Deletes++; if (DeleteFailure is not null) throw DeleteFailure;
                    Check(SelectionMode == 0 && cursor.List == 0 && cursor.Paragraph == 7 && cursor.Offset > 0, "Unsafe fake deletion.");
                    Tail = Tail.Remove(cursor.Offset - 1, 1); cursor = AfterDelete ?? (cursor with { Offset = cursor.Offset - 1 }); break;
                default: throw new Exception("Unexpected action: " + action);
            }
        }
        public void InsertMarker()
        {
            Inserts++; if (InsertFailure is not null) throw InsertFailure;
            Tail = Tail.Insert(cursor.Offset, CurrentParagraphStyle.Marker); cursor = AfterInsert ?? (cursor with { Offset = cursor.Offset + 1 });
        }
        public bool Select(ParagraphProbePosition begin, ParagraphProbePosition end) { Begin = begin; End = end; cursor = end; SelectionMode = BlockMode; return SelectResult; }
        public XDocument ReadBlock() { BlockReads++; if (ReadFailure is not null) throw ReadFailure; return Block; }
        public XDocument ReadDocument() { FullReads++; return Full; }
    }
}