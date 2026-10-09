using System.Reflection;
using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class FlatSnapshotTests
{
    private const BindingFlags PrivateStatic = BindingFlags.Static | BindingFlags.NonPublic;

    internal static void Run()
    {
        var document = HwpMarkup.Parse("""
            <HWPML><HEAD>
            <CHARSHAPELIST><CHARSHAPE Id="0"/><CHARSHAPE Id="1"><BOLD/></CHARSHAPE><CHARSHAPE Id="2"><ITALIC/></CHARSHAPE></CHARSHAPELIST>
            <PARASHAPELIST><PARASHAPE Id="0"><PARAMARGIN Left="120" Indent="0"/></PARASHAPE>
            <PARASHAPE Id="1" HeadingType="Bullet" Heading="7" Level="1"><PARAMARGIN Left="500" Indent="-50"/></PARASHAPE></PARASHAPELIST>
            <STYLELIST><STYLE Id="0" Type="Para" Name="Body" CharShape="0" ParaShape="0"/>
            <STYLE Id="1" Type="Para" Name="Reset" CharShape="0" ParaShape="0"/>
            <STYLE Id="2" Type="Para" Name="Code" CharShape="0" ParaShape="0"/>
            <STYLE Id="3" Type="Para" Name="Source" CharShape="0" ParaShape="0"/>
            <STYLE Id="4" Type="Para" Name="Caption" CharShape="0" ParaShape="0"/></STYLELIST>
            <BULLETLIST><BULLET Id="7" Char="●"><PARAHEAD CharShape="0"/></BULLET></BULLETLIST>
            </HEAD><BODY><SECTION>
            <P Style="0" ParaShape="0"><TEXT CharShape="0"><CHAR>unchanged cover</CHAR></TEXT></P>
            <P Style="0" ParaShape="0"><TEXT CharShape="0"><CHAR>한글 © 😀 </CHAR></TEXT><TEXT CharShape="1"><CHAR>굵게</CHAR></TEXT><TEXT CharShape="2"><CHAR>기울임</CHAR></TEXT></P>
            <P Style="0" ParaShape="1"><TEXT CharShape="0"><CHAR>목록</CHAR></TEXT></P>
            <P Style="0" ParaShape="0"><TEXT CharShape="0"><CHAR/></TEXT></P>
            </SECTION></BODY></HWPML>
            """);
        var profile = Profile(document);
        var styles = AuriPreviewStyleBindings.BindDocument(document, profile);
        var marker = new PreviewListMarker(1, "bullet", 1, 1, 1, true);
        var plan = new IrPreviewPlan("fixture", "fixture", new PreviewSummary(2, 2, 0, 0, 1),
            [new("text", "rich", ["한글 © 😀 굵게기울임"], ParagraphStyle: "body",
                FormattedLines: [[new("한글 © 😀 ", false, false), new("굵게", true, false), new("기울임", false, true)]]),
             new("text", "list", ["목록"], ParagraphStyle: "body", FormattedLines: [[new("목록", false, false)]], ListMarker: marker)], []);

        void Verify(XDocument snapshot)
        {
            var before = snapshot.ToString(SaveOptions.DisableFormatting);
            var parsed = HancomPreviewWriter.ReadParagraphs(snapshot);
            Check(Parity("VerifyText", snapshot, 1, () => HancomPreviewWriter.VerifyText(snapshot, plan, profile), plan, profile) is null, "Valid text snapshot failed.");
            var styleError = Parity("VerifyStyles", snapshot, 1, () => HancomPreviewWriter.VerifyStyles(parsed, plan, styles, 1), plan, styles, 1, null);
            Check(styleError is null, "Valid style snapshot failed: " + styleError?.Message);
            Check(Parity("VerifyCharacterMarks", snapshot, 1, () => HancomPreviewWriter.VerifyCharacterMarks(parsed, plan, styles, 1), plan, styles, 1) is null, "Valid character snapshot failed.");
            Check(Parity("VerifyLists", snapshot, 2, () => HancomPreviewWriter.VerifyLists(snapshot, parsed, plan, profile, 1), plan, profile, 1) is null, "Valid list snapshot failed.");
            Check(snapshot.ToString(SaveOptions.DisableFormatting) == before, "Shared flat verification mutated its snapshot.");
        }
        Verify(document);
        var parsed = HancomPreviewWriter.ReadParagraphs(document);
        Check(parsed[1].Text == "한글 © 😀 굵게기울임" && parsed[1].Runs.Count == 3 &&
            parsed[1].Runs[1].Bold && parsed[1].Runs[2].Italic, "Snapshot parsing lost Unicode or character marks.");
        Check(parsed[2].NativeList == new SavedNativeList("bullet", 1, 7, 500) &&
            parsed[2].LeftMargin == 500 && parsed[2].Indentation == -50, "Snapshot parsing lost native list identity or margins.");

        void Broken(string method, Action<XDocument> change, Action<XDocument> verify, int exports, params object?[] arguments)
        {
            var bad = new XDocument(document); change(bad);
            var before = bad.ToString(SaveOptions.DisableFormatting);
            var error = Parity(method, bad, exports, () => verify(bad), arguments);
            Check(error is not null, $"{method} accepted a corrupted snapshot.");
            Check(bad.ToString(SaveOptions.DisableFormatting) == before, "Failed verification mutated its snapshot.");
        }
        Broken("VerifyText", d => Roots(d)[1].Elements("TEXT").First().Element("CHAR")!.Value = "changed",
            d => HancomPreviewWriter.VerifyText(d, plan, profile), 1, plan, profile);
        Broken("VerifyStyles", d => Roots(d)[1].SetAttributeValue("Style", "1"),
            d => HancomPreviewWriter.VerifyStyles(HancomPreviewWriter.ReadParagraphs(d), plan, styles, 1), 1, plan, styles, 1, null);
        Broken("VerifyCharacterMarks", d => Roots(d)[1].Elements("TEXT").First().SetAttributeValue("CharShape", "1"),
            d => HancomPreviewWriter.VerifyCharacterMarks(HancomPreviewWriter.ReadParagraphs(d), plan, styles, 1), 1, plan, styles, 1);
        Broken("VerifyLists", d => d.Descendants("BULLET").Single().SetAttributeValue("Char", "○"),
            d => HancomPreviewWriter.VerifyLists(d, HancomPreviewWriter.ReadParagraphs(d), plan, profile, 1), 2, plan, profile, 1);

        // Original prototypes and generated clones must receive the same checks
        // whether supplied by the COM wrapper or by the shared snapshot path.
        ObjectSnapshots(document, profile, styles);
        HancomPreviewWriter.VerifyBoxes(document, plan, styles, null, 1);
        HancomPreviewWriter.VerifyCaptions(document, plan, styles, null, 1);
        Console.WriteLine("Shared flat snapshots preserve legacy text/style/marks/list/object checks, export boundaries and input immutability.");
    }

    private static void ObjectSnapshots(XDocument definitions, InvestigationTemplateProfile profile, AuriPreviewStyleBindings styles)
    {
        XElement Box(string content) => new("P", new XAttribute("Style", "0"), new XAttribute("ParaShape", "0"),
            new XElement("TEXT", new XAttribute("CharShape", "0"), new XElement("TABLE",
                new XElement("SHAPEOBJECT", new XElement("POSITION", new XAttribute("TreatAsChar", "true"))),
                new XElement("ROW", new XElement("CELL", new XElement("PARALIST", Paragraph(content, "2")))))));
        var boxDocument = WithRoots(definitions, Box("prototype"), Box("box content"));
        var boxRoot = Roots(boxDocument)[0];
        var box = Construct<AuriMinimalBoxPrototype>(0, Static<AuriMinimalBoxPrototype>("StableXml", boxRoot),
            Static<AuriMinimalBoxPrototype>("StableBoxStructureXml", boxRoot, styles, true), "unused native block");
        var boxPlan = new IrPreviewPlan("fixture", "fixture", new PreviewSummary(1, 0, 1, 0, 0), [new("code", "box", ["box content"])], []);
        ObjectParity("VerifyBoxes", boxDocument, () => HancomPreviewWriter.VerifyBoxes(boxDocument, boxPlan, styles, box, 1), boxPlan, styles, box, 1, true);
        var damagedBox = new XDocument(boxDocument); Roots(damagedBox)[1].Descendants("CELL").Single().AddAfterSelf(new XElement("CELL"));
        Check(Parity("VerifyBoxes", damagedBox, 1, () => HancomPreviewWriter.VerifyBoxes(damagedBox, boxPlan, styles, box, 1), boxPlan, styles, box, 1, true) is not null,
            "Shared box verification accepted a changed cell structure.");

        XElement Caption(string caption) => new("P", new XAttribute("Style", "4"), new XAttribute("ParaShape", "0"),
            new XElement("TEXT", new XAttribute("CharShape", "0"), new XElement("CHAR", "["),
                new XElement("AUTONUM", new XAttribute("NumberType", "Figure"), new XAttribute("Number", "1"),
                    new XElement("AUTONUMFORMAT", new XAttribute("Type", "Digit"))), new XElement("CHAR", "]" + caption)));
        var captionDocument = WithRoots(definitions, Caption("slot"), Paragraph("picture"), Caption("caption content"));
        var captionRoot = Roots(captionDocument)[0];
        var caption = Construct<AuriMinimalCaptionPrototype>(0, Static<AuriMinimalCaptionPrototype>("StableCaptionContentXml", captionRoot),
            Static<AuriMinimalCaptionPrototype>("StableCaptionStructureXml", captionRoot, styles), "unused native block");
        var figurePlan = new IrPreviewPlan("fixture", "fixture", new PreviewSummary(1, 0, 0, 1, 0), [new("figure", "figure", ["image", "caption content", ""])], []);
        ObjectParity("VerifyCaptions", captionDocument, () => HancomPreviewWriter.VerifyCaptions(captionDocument, figurePlan, styles, caption, 1), figurePlan, styles, caption, 1, true);
        var damagedCaption = new XDocument(captionDocument); Roots(damagedCaption)[2].Descendants("AUTONUM").Single().SetAttributeValue("NumberType", "Table");
        Check(Parity("VerifyCaptions", damagedCaption, 1, () => HancomPreviewWriter.VerifyCaptions(damagedCaption, figurePlan, styles, caption, 1), figurePlan, styles, caption, 1, true) is not null,
            "Shared caption verification accepted a changed native number kind.");
        Check(Parity("VerifyBoxes", boxDocument, 0, () => HancomPreviewWriter.VerifyBoxes(boxDocument, boxPlan, styles, null, 1), boxPlan, styles, null, 1, true) is not null,
            "A required box prototype was not checked before export.");
        Check(Parity("VerifyCaptions", captionDocument, 0, () => HancomPreviewWriter.VerifyCaptions(captionDocument, figurePlan, styles, null, 1), figurePlan, styles, null, 1, true) is not null,
            "A required caption prototype was not checked before export.");
    }

    private static void ObjectParity(string method, XDocument document, Action pure, params object?[] arguments)
    {
        var before = document.ToString(SaveOptions.DisableFormatting);
        Check(Parity(method, document, 1, pure, arguments) is null, $"Valid {method} snapshot failed.");
        Check(document.ToString(SaveOptions.DisableFormatting) == before, "Object comparison mutated the shared snapshot.");
    }

    private static Exception? Parity(string method, XDocument document, int expectedExports, Action pure, params object?[] arguments)
    {
        var fake = new FlatSnapshotAutomation(document.ToString(SaveOptions.DisableFormatting));
        var wrapper = typeof(HancomPreviewWriter).GetMethods(PrivateStatic).Single(m => m.Name == method && m.GetParameters()[0].ParameterType == typeof(object));
        var legacy = Outcome(() => wrapper.Invoke(null, new object?[] { fake }.Concat(arguments).ToArray()));
        var shared = Outcome(pure);
        Check(legacy?.GetType() == shared?.GetType() && legacy?.Message == shared?.Message, $"{method} changed legacy acceptance or diagnostics.");
        Check(fake.Exports == expectedExports, $"{method} changed the legacy export count: {fake.Exports}/{expectedExports}.");
        return shared;
    }
    private static Exception? Outcome(Action action) { try { action(); return null; } catch (TargetInvocationException e) { return e.InnerException!; } catch (Exception e) { return e; } }
    private static T Construct<T>(params object[] values) => (T)typeof(T).GetConstructors(BindingFlags.Instance | BindingFlags.NonPublic).Single().Invoke(values);
    private static string Static<T>(string name, params object[] values) => (string)typeof(T).GetMethod(name, PrivateStatic)!.Invoke(null, values)!;
    private static XElement Paragraph(string text, string style = "0") => new("P", new XAttribute("Style", style), new XAttribute("ParaShape", "0"), new XElement("TEXT", new XAttribute("CharShape", "0"), new XElement("CHAR", text)));
    private static XElement[] Roots(XDocument document) => document.Descendants("SECTION").Single().Elements("P").ToArray();
    private static XDocument WithRoots(XDocument definitions, params XElement[] roots) => new(new XElement("HWPML", new XElement(definitions.Root!.Element("HEAD")!), new XElement("BODY", new XElement("SECTION", roots))));
    private static InvestigationTemplateProfile Profile(XDocument document)
    {
        var source = new TemplateSource(Paragraph("source"), "source", "", "");
        return InvestigationTemplateProfile.FromTaggedTemplate(typeof(IrPreviewPlan).Assembly.Location,
            [new("body", "Body"), new("code.anchor", "Body"), new("code", "Code"), new("code.source", "Source"), new("figure.caption", "Caption")],
            "Reset", 142, 6, 380, new("figure.caption", "[", "]slot", "slot"), source, source,
            new("bullet", 0, new XElement(document.Descendants("BULLET").Single())), new("ordered", 0, new XElement("NUMBERING")));
    }
    private static void Check(bool condition, string message) { if (!condition) throw new Exception(message); }
}

// Public so the backend's legacy dynamic binding can exercise this fake export
// without creating any COM instance or interacting with Hancom.
public sealed class FlatSnapshotAutomation(string markup)
{
    public int Exports { get; private set; }
    public string GetTextFile(string format, string options)
    {
        if (format != "HWPML2X" || options != "") throw new Exception("Unexpected native operation in verification.");
        Exports++; return markup;
    }
}
