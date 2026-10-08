using System.Text.Json;
using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

static void Check(bool condition, string message) { if (!condition) throw new Exception(message); }
static void Reject(Action operation) {
    try { operation(); } catch (InvalidDataException) { return; }
    throw new Exception("Expected rejection.");
}
var sectionFixture = XDocument.Parse("<HWPML><BODY><SECTION><P Style='0' PageBreak='false' ColumnBreak='false'><TEXT CharShape='1'><COLDEF/><SECDEF><MASTERPAGE Type='Even'><PARALIST><P><TEXT><LINE/><CHAR/></TEXT></P></PARALIST></MASTERPAGE></SECDEF><CHAR>{{md2hwp:begin:template}}</CHAR></TEXT></P><P><TEXT><CHAR>{{md2hwp:ir-version:0.4}}</CHAR></TEXT></P></SECTION></BODY></HWPML>");
var originalSectionFixture = sectionFixture.ToString();
var retainedSection = new XElement(sectionFixture.Descendants("SECDEF").Single());
var separatedSection = TaggedTemplateBinding.PreserveBeginSectionSettings(sectionFixture);
var sectionRoots = AuriMinimalBoxPrototype.RootParagraphs(separatedSection);
Check(sectionFixture.ToString()==originalSectionFixture,"Section-boundary normalization mutated source.");
Check(sectionRoots.Count==3 && TaggedTemplateBinding.DirectText(sectionRoots[0])=="" && TaggedTemplateBinding.DirectText(sectionRoots[1])=="{{md2hwp:begin:template}}","Section settings not separated from the declaration.");
Check(XNode.DeepEquals(retainedSection,sectionRoots[0].Descendants("SECDEF").Single()),"Section or master-page settings changed.");
Check(!sectionRoots[1].Descendants("SECDEF").Any() && !sectionRoots[1].Descendants("P").Any(),"Section settings retained in disposable declaration.");
Check(sectionRoots[1].Attribute("PageBreak") is null && sectionRoots[1].Attribute("ColumnBreak") is null,"False break flags retained in noninitial declaration.");
Check(XNode.DeepEquals(separatedSection,TaggedTemplateBinding.PreserveBeginSectionSettings(separatedSection)),"Section normalization is not idempotent.");
foreach (var tag in new[]{"end:template", "body", "begin:heading1", "begin:figure.caption"}) {
    var other=new XDocument(sectionFixture);
    other.Descendants("CHAR").Last().Value="unchanged";
    var otherFirst=AuriMinimalBoxPrototype.RootParagraphs(other)[0];
    otherFirst.Elements("TEXT").Elements("CHAR").Single().Value=TaggedTemplateBinding.Tag(tag);
    Check(XNode.DeepEquals(other,TaggedTemplateBinding.PreserveBeginSectionSettings(other)),"Section exception leaked into another declaration.");
}
var splitSection = new XDocument(sectionFixture);
var splitFirst = AuriMinimalBoxPrototype.RootParagraphs(splitSection)[0];
splitFirst.Elements("TEXT").Elements("CHAR").Remove();
splitFirst.Add(new XElement("TEXT",new XAttribute("CharShape","2"),new XElement("CHAR","{{md2hwp:begin:")),new XElement("TEXT",new XAttribute("CharShape","3"),new XElement("CHAR","template}}")));
var splitSectionRoots=AuriMinimalBoxPrototype.RootParagraphs(TaggedTemplateBinding.PreserveBeginSectionSettings(splitSection));
Check(TaggedTemplateBinding.DirectText(splitSectionRoots[0])=="" && TaggedTemplateBinding.DirectText(splitSectionRoots[1])=="{{md2hwp:begin:template}}","Split declaration runs leaked into retained section paragraph.");
Check(splitSectionRoots[1].Elements("TEXT").All(e=>e.HasElements),"Empty control-only run retained in declaration.");
var unsupportedSection = new XDocument(sectionFixture);
AuriMinimalBoxPrototype.RootParagraphs(unsupportedSection)[0].Element("TEXT")!.Add(new XElement("TABLE"));
Check(XNode.DeepEquals(unsupportedSection,TaggedTemplateBinding.PreserveBeginSectionSettings(unsupportedSection)),"Unrelated table control accepted on begin:template.");
Console.WriteLine("Begin-template section preservation checks passed.");
static XElement HeadingParagraph(string text) => new("P", new XAttribute("Style", "0"), new XAttribute("ParaShape", "0"), new XElement("TEXT", new XAttribute("CharShape", "0"), new XElement("CHAR", text)));
var headingFixture = new XDocument(new XElement("HWPML",new XElement("BODY",new XElement("SECTION",new[]{"begin:template","begin:heading1","slot:heading1","begin:each.child:heading2","slot:heading2","end:each.child:heading2","end:heading1","end:template"}.Select(t=>HeadingParagraph(TaggedTemplateBinding.Tag(t)))))));
var loweredHeading = TemplateHeadingBlocks.Lower(headingFixture);
Check(AuriMinimalBoxPrototype.RootParagraphs(loweredHeading.Document).Any(p=>TaggedTemplateBinding.DirectText(p)=="{{md2hwp:heading1}}"),"heading1 block was not lowered to the new role.");
var repeatFixture = new[]{"slot:heading1","begin:each.child:heading2","slot:heading2","end:each.child:heading2"}.Select(t=>HeadingParagraph(TaggedTemplateBinding.Tag(t))).ToArray();
var headingOperations = new[]{new PreviewOperation("text","parent",[],ParagraphStyle:"heading1"),new PreviewOperation("text","child A",[],ParagraphStyle:"heading2"),new PreviewOperation("text","nested",[],ParagraphStyle:"heading3"),new PreviewOperation("text","child B",[],ParagraphStyle:"heading2"),new PreviewOperation("text","next parent",[],ParagraphStyle:"heading1"),new PreviewOperation("text","other child",[],ParagraphStyle:"heading2")};
var headingPlan = new IrPreviewPlan("fixture","fixture",new PreviewSummary(6,6,0,0,0),headingOperations,[]);
var repeatedHeadings=TemplateHeadingEach.Expand(repeatFixture,1,0,headingPlan,(p,operation,_)=>p.Element("TEXT")!.Element("CHAR")!.Value=operation.Label);
Check(repeatedHeadings.Select(TaggedTemplateBinding.DirectText).SequenceEqual(new[]{"parent","child A","child B"}),"heading2 child repetition parsed the wrong heading level or crossed the next heading1.");
Reject(()=>TemplateHeadingEach.Validate(new[]{HeadingParagraph("{{md2hwp:slot:heading.1}}")},1));
var attachFixture=new XDocument(headingFixture);
var headingDefinitions=new XElement("HEAD",new XElement("PARASHAPELIST",new XElement("PARASHAPE",new XAttribute("Id","0"))),new XElement("CHARSHAPELIST",new XElement("CHARSHAPE",new XAttribute("Id","0"))));
attachFixture.Root!.AddFirst(headingDefinitions);
var attachLayout=TemplateHeadingBlocks.Lower(attachFixture).Layout;
var renderedHeading=new XDocument(new XElement("HWPML",new XElement(headingDefinitions),new XElement("BODY",new XElement("SECTION",HeadingParagraph("generated title")))));
var attachPlan=new IrPreviewPlan("fixture","fixture",new PreviewSummary(1,1,0,0,0),new[]{new PreviewOperation("text","title",new[]{"attached title"},ParagraphStyle:"heading1")},[]);
Check(TaggedTemplateBinding.DirectText(AuriMinimalBoxPrototype.RootParagraphs(attachLayout.Attach(renderedHeading,attachPlan,0)).Single())=="attached title","heading1 block attachment did not parse and fill its title.");
Console.WriteLine("Heading role, block attachment and child repetition checks passed.");
var startMetadata=new Dictionary<string,string>{{"md2hwp-heading1-start","3"}};
Check(TemplateHeadingNumbers.Start(startMetadata)==3 && TemplateHeadingNumbers.Start(new Dictionary<string,string>())==1,"Heading1 start setting.");
foreach(var invalidStart in new[]{"0","-1","1.5","2147483648"," 3","+3"}) Reject(()=>TemplateHeadingNumbers.Start(new Dictionary<string,string>{{"md2hwp-heading1-start",invalidStart}}));
var trackedChapters=TemplateHeadingNumbers.Track(headingOperations,3);
Check(trackedChapters.Select(o=>o.Heading1Number).SequenceEqual(new int?[]{3,3,3,3,4,4}),"Heading1 tracking incremented child headings.");
Check(TemplateHeadingNumbers.Track(new[]{new PreviewOperation("figure","figure",[])},3).Single().Heading1Number is null,"Figure before first heading1 acquired a chapter.");
Reject(()=>TemplateHeadingNumbers.Track(headingOperations,int.MaxValue));
var chapterSlots=XDocument.Parse("<DOC><P><TEXT CharShape='4'><CHAR>Chapter {{md2hwp:num:head</CHAR></TEXT><TEXT CharShape='5'><CHAR>ing1}} 끝</CHAR></TEXT></P></DOC>");
var preparedChapterSlots=TemplateHeadingNumbers.Prepare(chapterSlots);
var filledChapterSlots=TemplateHeadingNumbers.Fill(preparedChapterSlots.Root!.Elements(),3);
Check(TaggedTemplateBinding.DirectText(filledChapterSlots.Single())=="Chapter 3 끝","Split-run heading1 number slot.");
Check((string?)filledChapterSlots.Single().Element("TEXT")!.Attribute("CharShape")=="4","Heading1 number formatting changed.");
Reject(()=>TemplateHeadingNumbers.Fill(preparedChapterSlots.Root!.Elements(),null));
Reject(()=>TemplateHeadingNumbers.RequireResolved(preparedChapterSlots));
var unresolvedText=TaggedTemplateBinding.DirectText(preparedChapterSlots.Root!.Elements().Single());
var splitUnresolved=new XDocument(new XElement("DOC",new XElement("P",new XElement("TEXT",new XElement("CHAR",unresolvedText[..(unresolvedText.Length/2)])),new XElement("TEXT",new XElement("CHAR",unresolvedText[(unresolvedText.Length/2)..])))));
Reject(()=>TemplateHeadingNumbers.RequireResolved(splitUnresolved));
var nativeCaptionRoots=new List<XElement>();
foreach(var chapter in new[]{3,3,4,4}) {
    nativeCaptionRoots.Add(new XElement("P",new XElement("TEXT",new XAttribute("CharShape","0"),new XElement("PICTURE",new XElement("SHAPEOBJECT",new XElement("SIZE",new XAttribute("Width","10000")))))));
    var paragraph=HeadingParagraph("[그림 "+TemplateHeadingNumbers.Tag+"-");
    paragraph.Element("TEXT")!.Add(new XElement("AUTONUM",new XAttribute("NumberType","Figure"),new XAttribute("Number","99")),new XElement("CHAR","] caption"));
    nativeCaptionRoots.Add(paragraph);
}
var fakeCaptionDocument=TemplateHeadingNumbers.Prepare(new XDocument(new XElement("HWPML",new XElement("BODY",new XElement("SECTION",nativeCaptionRoots)))));
var captionPlan=new IrPreviewPlan("fixture","fixture",new PreviewSummary(4,0,0,4,0),new[]{3,3,4,4}.Select(n=>new PreviewOperation("figure","figure",new[]{"image","caption",""},Heading1Number:n)).ToArray(),[]);
var captionLayout=new NativeFigureCaption(new XElement("CAPTION",new XAttribute("Side","Bottom"),new XElement("PARALIST")));
var chapterPictures=captionLayout.Attach(fakeCaptionDocument,captionPlan,0);
TemplateHeadingBlocks.RecalculateFigureNumbers(chapterPictures);
TemplateHeadingNumbers.RequireResolved(chapterPictures);
Check(chapterPictures.Descendants("NEWNUM").Count()==2,"Figure counters must restart once per chapter.");
Check(chapterPictures.Descendants("AUTONUM").Select(e=>(string)e.Attribute("Number")!).SequenceEqual(new[]{"1","2","1","2"}),"Figure counters did not restart at the second chapter.");
Check(chapterPictures.Descendants("CAPTION").Select(c=>TaggedTemplateBinding.DirectText(c.Descendants("P").Single())).SequenceEqual(new[]{"[그림 3-] caption","[그림 3-] caption","[그림 4-] caption","[그림 4-] caption"}),"Caption chapter numbers were not filled from the figure operation.");
Console.WriteLine("Heading1 start, number slots and native figure restart checks passed.");
var metadata = new Dictionary<string,string> {
    ["title"]="문서 제목", ["date"]="2026년 1월 2일", ["date-meta"]="2026-01-02",
    ["md2hwp-report-number"]="기본 2026-01", ["md2hwp-literal"]="{{md2hwp:meta:title}}"
};
var input = XDocument.Parse("<DOC><P Style='1'><TEXT CharShape='4'><CHAR>표지 {{md2hwp:me</CHAR></TEXT><TEXT CharShape='5'><CHAR>ta:title}} / {{md2hwp:meta:date-meta:fmt:%Y년 %-m월 %-d일}} 끝</CHAR></TEXT></P><P><TEXT><TABLE><P><TEXT CharShape='6'><CHAR>{{md2hwp:meta:md2hwp-report-number}}</CHAR></TEXT></P></TABLE><CHAR>{{md2hwp:meta:md2hwp-literal}}</CHAR></TEXT></P></DOC>");
var splitOnly = XDocument.Parse("<DOC><P><TEXT CharShape='4'><CHAR>{{md2hwp:meta:</CHAR></TEXT><TEXT CharShape='5'><CHAR>title}}</CHAR></TEXT></P></DOC>");
var splitResult = TemplateMetadata.Apply(splitOnly,metadata);
Check(splitResult.Descendants("TEXT").Count()==1 && splitResult.Descendants("CHAR").Single().Value=="문서 제목","Consumed empty formatting run retained.");
var original = input.ToString();
var prepared = TemplateMetadata.Prepare(input,metadata);
Check(!prepared.Document.ToString().Contains("{{md2hwp:"),"Literal metadata reached template binding.");
Check(XNode.DeepEquals(prepared.Restore(prepared.Document),TemplateMetadata.Apply(input,metadata)),"Deferred restoration differs.");
var output = TemplateMetadata.Apply(input,metadata);
Check(input.ToString()==original,"Source mutated.");
var first = output.Descendants("P").First();
Check(TaggedTemplateBinding.DirectText(first)=="표지 문서 제목 / 2026년 1월 2일 끝","Split-run replacement.");
Check(first.Elements("TEXT").First().Element("CHAR")!.Value=="표지 문서 제목","First-run formatting lost.");
Check(output.Descendants("TABLE").Descendants("CHAR").Single().Value=="기본 2026-01","Nested cell replacement.");
Check(output.Descendants("P").Skip(1).First().Element("TEXT")!.Elements("CHAR").Single().Value=="{{md2hwp:meta:title}}","Replacement recursively interpreted.");
Check(TemplateMetadata.FormatDate("2026-01-02","%Y/%m/%d %y %-m %-d %F %%")=="2026/01/02 26 1 2 2026-01-02 %","fmt output.");
foreach (var fmt in new[]{"", "%", "%Q", "%H", "%-Y", "%n", "line\n"})
    Reject(()=>TemplateMetadata.FormatDate("2026-01-02",fmt));
Reject(()=>TemplateMetadata.Apply(input,new Dictionary<string,string>()));
Reject(()=>TemplateMetadata.Apply(XDocument.Parse("<DOC><P><TEXT><CHAR>{{md2hwp:meta:title:fmt:%Y}}</CHAR></TEXT></P></DOC>"),metadata));
Reject(()=>TemplateMetadata.Apply(XDocument.Parse("<DOC><P><TEXT><CHAR>{{md2hwp:meta:ti</CHAR><AUTONUM/><CHAR>tle}}</CHAR></TEXT></P></DOC>"),metadata));
using var valid = JsonDocument.Parse("{\"author\":[\"홍길동\",\"김연구\"],\"date\":\"미정\",\"md2hwp-test\":\"값\"}");
Check(TemplateMetadata.Read(valid.RootElement)["author"]=="홍길동, 김연구","Author joining.");
foreach(var invalid in new[]{"{\"lang\":\"ko-KR\"}","{\"date-meta\":\"2026-02-29\",\"date\":\"2026년 2월 29일\"}","{\"author\":[]}","{\"author\":[\"\",\"\"]}"}) {
    using var value=JsonDocument.Parse(invalid); Reject(()=>TemplateMetadata.Read(value.RootElement));
}
Console.WriteLine("Backend metadata/fmt contract checks passed.");

static XElement BoxParagraph(string text, string style = "0", string paraShape = "0", string charShape = "0") =>
    new("P", new XAttribute("Style", style), new XAttribute("ParaShape", paraShape),
        new XElement("TEXT", new XAttribute("CharShape", charShape), new XElement("CHAR", text)));
static XDocument BoxTemplate(bool withTitle, bool splitSlots = false, bool sameStyle = false)
{
    XElement Slot(string name, bool title)
    {
        var p = BoxParagraph(TaggedTemplateBinding.Tag("slot:box." + name), title && !sameStyle ? "1" : "0", title ? "1" : "0", title ? "2" : "0");
        if (splitSlots)
        {
            var text = TaggedTemplateBinding.DirectText(p);
            p.Element("TEXT")!.Element("CHAR")!.Value = text[..10];
            p.Add(new XElement("TEXT", new XAttribute("CharShape", "3"), new XElement("CHAR", text[10..])));
        }
        return p;
    }
    var paragraphs = new List<XElement>();
    if (withTitle) paragraphs.Add(Slot("title", true));
    paragraphs.Add(Slot("content", false));
    return new XDocument(new XElement("HWPML",
        new XElement("HEAD",
            new XElement("PARASHAPELIST", new XAttribute("Count", "2"),
                new XElement("PARASHAPE", new XAttribute("Id", "0"), new XElement("PARAMARGIN", new XAttribute("Left", "0"))),
                new XElement("PARASHAPE", new XAttribute("Id", "1"), new XElement("PARAMARGIN", new XAttribute("Left", "80")))),
            new XElement("CHARSHAPELIST", new XAttribute("Count", "4"),
                Enumerable.Range(0, 4).Select(i => new XElement("CHARSHAPE", new XAttribute("Id", i), new XAttribute("Height", 1000 + i * 100)))),
            new XElement("STYLELIST", new XAttribute("Count", "2"), Enumerable.Range(0, 2).Select(i =>
                new XElement("STYLE", new XAttribute("Id", i), new XAttribute("Type", "Para"), new XAttribute("Name", "box-fixture-" + i),
                    new XAttribute("ParaShape", i), new XAttribute("CharShape", i == 0 ? 0 : 2), new XAttribute("NextStyle", "0"))))),
        new XElement("BODY", new XElement("SECTION",
            BoxParagraph(TaggedTemplateBinding.Tag("begin:block.box")),
            new XElement("P", new XAttribute("Style", "0"), new XAttribute("ParaShape", "0"),
                new XElement("TEXT", new XAttribute("CharShape", "0"), new XElement("TABLE",
                    new XElement("SHAPEOBJECT", new XAttribute("InstId", "box-contract"), new XElement("SIZE", new XAttribute("Width", "10000"), new XAttribute("Height", "1000")),
                        new XElement("POSITION", new XAttribute("TreatAsChar", "true")),
                        new XElement("CAPTION", new XAttribute("Side", "Bottom"), new XElement("PARALIST", BoxParagraph("출처: fixture source")))),
                    new XElement("ROW", new XElement("CELL", new XAttribute("Width", "10000"), new XAttribute("Height", "1000"),
                        new XElement("PARALIST", paragraphs)))))),
            BoxParagraph(TaggedTemplateBinding.Tag("end:block.box"))))));
}
static XElement[] BoxCellParagraphs(XElement root) => root.Descendants("CELL").Single().Element("PARALIST")!.Elements("P").ToArray();
static XElement BoxRenderedRoot(XDocument lowered, IReadOnlyList<string> lines)
{
    var root = new XElement(AuriMinimalBoxPrototype.RootParagraphs(lowered)[1]);
    var body = BoxCellParagraphs(root).Single();
    var nodes = new List<object>();
    for (var i = 0; i < lines.Count; i++)
    {
        if (i > 0) nodes.Add(new XElement("LINEBREAK"));
        nodes.Add(new XText(lines[i]));
    }
    body.ReplaceNodes(new XElement("TEXT", new XAttribute("CharShape", "0"), new XElement("CHAR", nodes)));
    return root;
}
static XDocument BoxRenderedDocument(XDocument lowered, params XElement[] roots) =>
    new(new XElement("HWPML", new XElement(lowered.Root!.Element("HEAD")!), new XElement("BODY", new XElement("SECTION", roots))));
static IrPreviewPlan BoxPlan(IReadOnlyList<string> lines) =>
    new("fixture", "fixture", new PreviewSummary(1, 0, 1, 0, 0), new[] { new PreviewOperation("box", "verbatim_block", lines, ParagraphStyle: "block.box") }, []);
static XElement[] AttachBoxLines(TemplateBoxParagraphs layout, XDocument lowered, IReadOnlyList<string> lines)
{
    var rendered = BoxRenderedDocument(lowered, BoxRenderedRoot(lowered, lines));
    var original = rendered.ToString();
    var originalLines = lines.ToArray();
    var attached = layout.Attach(rendered, BoxPlan(lines), 0);
    Check(rendered.ToString() == original, "Box attachment mutated the intermediate document.");
    Check(lines.SequenceEqual(originalLines), "Box attachment mutated IR lines.");
    Check(XNode.DeepEquals(rendered.Descendants("CAPTION").Single(), attached.Descendants("CAPTION").Single()), "Box paragraph attachment changed the native source caption.");
    Check(attached.Descendants("CELL").Single().Attribute("Width")!.Value == "10000", "Box paragraph attachment changed cell width.");
    return BoxCellParagraphs(AuriMinimalBoxPrototype.RootParagraphs(attached).Single());
}
var boxFixture = BoxTemplate(withTitle: true, splitSlots: true);
var originalBoxFixture = boxFixture.ToString();
var loweredBox = TemplateBoxParagraphs.Lower(boxFixture);
Check(boxFixture.ToString() == originalBoxFixture, "Box lowering mutated the source template.");
Check(BoxCellParagraphs(AuriMinimalBoxPrototype.RootParagraphs(loweredBox.Document)[1]).Length == 1, "Box lowering retained the optional title sample.");
Check(TaggedTemplateBinding.DirectText(BoxCellParagraphs(AuriMinimalBoxPrototype.RootParagraphs(loweredBox.Document)[1]).Single()) == TaggedTemplateBinding.Tag("slot:box.content"), "Box lowering changed the content slot.");
var boxLines = new[] { "제목: 건축물의 사용승인", "", " \t", "제22조 ① 원문", "  ② 둘째\t열  ", "", "제목: 본문 리터럴", "{{md2hwp:meta:title}}", "" };
var boxParagraphs = AttachBoxLines(loweredBox.Layout, loweredBox.Document, boxLines);
Check(boxParagraphs.Select(TaggedTemplateBinding.DirectText).SequenceEqual(new[] { "건축물의 사용승인", "제22조 ① 원문", "  ② 둘째\t열  ", "", "제목: 본문 리터럴", "{{md2hwp:meta:title}}", "" }), "Box title extraction or verbatim paragraphs lost text, indentation, tabs, internal blank lines or literal tags.");
Check((string?)boxParagraphs[0].Attribute("Style") == "1" && (string?)boxParagraphs[0].Attribute("ParaShape") == "1", "Box title did not retain its template paragraph formatting.");
Check((string?)boxParagraphs[0].Element("TEXT")!.Attribute("CharShape") == "2", "Box title did not inherit the first slot character's formatting.");
Check(boxParagraphs.Skip(1).All(p => (string?)p.Attribute("Style") == "0" && (string?)p.Attribute("ParaShape") == "0" && (string?)p.Element("TEXT")!.Attribute("CharShape") == "0"), "Box content paragraphs did not retain the content slot formatting.");
Check(boxParagraphs.All(p => !p.Descendants("LINEBREAK").Any()), "Box lines remained line-break controls instead of separate paragraphs.");
foreach (var lines in new[] { new[] { "plain first line", "", "제목: body literal" }, new[] { " 제목: leading-space literal", "" }, new[] { "" } })
    Check(AttachBoxLines(loweredBox.Layout, loweredBox.Document, lines).Select(TaggedTemplateBinding.DirectText).SequenceEqual(lines), "A non-title box line or blank paragraph was interpreted or removed.");
Check(AttachBoxLines(loweredBox.Layout, loweredBox.Document, new[] { "제목:  leading space", "body" })[0].Value == " leading space", "Title extraction removed more than one optional ASCII space.");
Check(AttachBoxLines(loweredBox.Layout, loweredBox.Document, new[] { "제목:공백 없음", "body" })[0].Value == "공백 없음", "Title extraction required a space after the colon.");
Check(AttachBoxLines(loweredBox.Layout, loweredBox.Document, new[] { "제목: 제목만", "", " \t" }).Select(TaggedTemplateBinding.DirectText).SequenceEqual(new[] { "제목만" }), "A title-only box retained an empty content paragraph.");
foreach (var emptyTitle in new[] { "제목:", "제목: " })
    Reject(() => AttachBoxLines(loweredBox.Layout, loweredBox.Document, new[] { emptyTitle, "body" }));
Check(AttachBoxLines(loweredBox.Layout, loweredBox.Document, new[] { "제목: {{md2hwp:meta:title}}", "body" })[0].Value == "{{md2hwp:meta:title}}", "Manuscript title text was recursively interpreted as a template tag.");
var remappedRendered = BoxRenderedDocument(loweredBox.Document, BoxRenderedRoot(loweredBox.Document, new[] { "제목: 서식 번호 재배치", "body" }));
foreach (var definition in remappedRendered.Descendants("PARASHAPE")) definition.SetAttributeValue("Id", (int)definition.Attribute("Id")! + 10);
foreach (var definition in remappedRendered.Descendants("CHARSHAPE")) definition.SetAttributeValue("Id", (int)definition.Attribute("Id")! + 20);
foreach (var attribute in remappedRendered.Descendants().Attributes().Where(a => a.Name.LocalName == "ParaShape")) attribute.Value = (int.Parse(attribute.Value) + 10).ToString();
foreach (var attribute in remappedRendered.Descendants().Attributes().Where(a => a.Name.LocalName == "CharShape")) attribute.Value = (int.Parse(attribute.Value) + 20).ToString();
var remappedAttached = loweredBox.Layout.Attach(remappedRendered, BoxPlan(new[] { "제목: 서식 번호 재배치", "body" }), 0);
var remappedParagraphs = BoxCellParagraphs(AuriMinimalBoxPrototype.RootParagraphs(remappedAttached).Single());
Check((string?)remappedParagraphs[0].Attribute("ParaShape") == "11" && (string?)remappedParagraphs[0].Element("TEXT")!.Attribute("CharShape") == "22", "Title copied stale source formatting IDs into the saved document.");
Check((string?)remappedParagraphs[1].Attribute("ParaShape") == "10" && (string?)remappedParagraphs[1].Element("TEXT")!.Attribute("CharShape") == "20", "Content copied stale source formatting IDs into the saved document.");
var noTitleBox = TemplateBoxParagraphs.Lower(BoxTemplate(withTitle: false));
Check(AttachBoxLines(noTitleBox.Layout, noTitleBox.Document, boxLines).Select(TaggedTemplateBinding.DirectText).SequenceEqual(boxLines), "A template without a title slot consumed the title prefix or following blank lines.");
var sameStyleBox = TemplateBoxParagraphs.Lower(BoxTemplate(withTitle: true, sameStyle: true));
var sameStyleParagraphs = AttachBoxLines(sameStyleBox.Layout, sameStyleBox.Document, new[] { "제목: 같은 스타일", "body" });
Check(sameStyleParagraphs.Select(TaggedTemplateBinding.DirectText).SequenceEqual(new[] { "같은 스타일", "body" }) && sameStyleParagraphs.All(p => (string?)p.Attribute("Style") == "0"), "Title and content slots sharing a native style were confused.");
foreach (var invalidKind in new[] { "duplicate", "reversed", "caption", "outside", "control" })
{
    var invalid = BoxTemplate(withTitle: true);
    var cell = invalid.Descendants("CELL").Single().Element("PARALIST")!;
    var title = cell.Elements("P").First();
    switch (invalidKind)
    {
        case "duplicate": cell.AddFirst(new XElement(title)); break;
        case "reversed": title.Remove(); cell.Add(title); break;
        case "caption": title.Remove(); invalid.Descendants("CAPTION").Single().Element("PARALIST")!.Add(title); break;
        case "outside": title.Remove(); invalid.Descendants("SECTION").Single().AddFirst(title); break;
        case "control": title.Element("TEXT")!.AddFirst(new XElement("AUTONUM", new XAttribute("NumberType", "Figure"), new XAttribute("Number", "1"))); break;
    }
    Reject(() => TemplateBoxParagraphs.Lower(invalid));
}
var mixedLinesA = new[] { "제목: mixed A", "", "first body" };
var mixedLinesB = new[] { "second body", "", "last body" };
var mixedOperations = new[] {
    new PreviewOperation("text", "before", new[] { "before" }, ParagraphStyle: "body"),
    new PreviewOperation("figure", "without source", new[] { "image", "caption A", "" }),
    new PreviewOperation("box", "first box", mixedLinesA, ParagraphStyle: "block.box"),
    new PreviewOperation("figure", "with source", new[] { "image", "caption B", "source" }),
    new PreviewOperation("box", "second box", mixedLinesB, ParagraphStyle: "block.box"),
    new PreviewOperation("text", "after", new[] { "after" }, ParagraphStyle: "body")
};
var mixedPlan = new IrPreviewPlan("fixture", "fixture", new PreviewSummary(6, 2, 2, 2, 0), mixedOperations, []);
var mixedRendered = BoxRenderedDocument(loweredBox.Document,
    BoxParagraph("static prefix"), BoxParagraph("before"), BoxParagraph("picture A"), BoxParagraph("caption A"),
    BoxRenderedRoot(loweredBox.Document, mixedLinesA), BoxParagraph("picture B"), BoxParagraph("caption B"), BoxParagraph("source"),
    BoxRenderedRoot(loweredBox.Document, mixedLinesB), BoxParagraph("after"));
var mixedOriginal = new XDocument(mixedRendered);
var mixedAttached = loweredBox.Layout.Attach(mixedRendered, mixedPlan, 1);
var mixedBeforeRoots = AuriMinimalBoxPrototype.RootParagraphs(mixedOriginal);
var mixedAfterRoots = AuriMinimalBoxPrototype.RootParagraphs(mixedAttached);
Check(mixedAfterRoots.Count == mixedBeforeRoots.Count, "Box attachment changed root paragraph count.");
Check(BoxCellParagraphs(mixedAfterRoots[4]).Select(TaggedTemplateBinding.DirectText).SequenceEqual(new[] { "mixed A", "first body" }), "First box operation mapped to the wrong root after a sourceless figure.");
Check(BoxCellParagraphs(mixedAfterRoots[8]).Select(TaggedTemplateBinding.DirectText).SequenceEqual(mixedLinesB), "Second box operation mapped to the wrong root after a sourced figure.");
foreach (var i in new[] { 0, 1, 2, 3, 5, 6, 7, 9 })
    Check(XNode.DeepEquals(mixedBeforeRoots[i], mixedAfterRoots[i]), "Box attachment changed a static, text or figure root.");
Check(XNode.DeepEquals(mixedRendered, mixedOriginal), "Mixed box attachment mutated its input.");
var boxLayoutExpected = loweredBox.Layout.Attach(
    BoxRenderedDocument(loweredBox.Document, BoxRenderedRoot(loweredBox.Document, new[] { "제목: 높이", "body" })),
    BoxPlan(new[] { "제목: 높이", "body" }), 0);
var staticTable = new XElement(boxLayoutExpected.Descendants("TABLE").Single());
staticTable.Element("SHAPEOBJECT")!.SetAttributeValue("InstId", "static-table");
boxLayoutExpected.Descendants("SECTION").Single().AddFirst(new XElement("P", new XElement("TEXT", staticTable)));
loweredBox.Layout.RecordLayout(boxLayoutExpected);
var boxLayoutActual = new XDocument(boxLayoutExpected);
foreach (var size in boxLayoutActual.Descendants("SIZE")) size.SetAttributeValue("Height", "2000");
foreach (var cell in boxLayoutActual.Descendants("CELL")) cell.SetAttributeValue("Height", "2000");
boxLayoutActual.Descendants("TABLE").Last().Element("SHAPEOBJECT")!.Element("SIZE")!.SetAttributeValue("Width", "9999");
boxLayoutActual.Descendants("TABLE").Last().Descendants("P").Last().SetAttributeValue("ParaShape", "1");
loweredBox.Layout.NormalizeLayout(boxLayoutExpected, boxLayoutActual);
Check(boxLayoutActual.Descendants("TABLE").Last().Element("SHAPEOBJECT")!.Element("SIZE")!.Attribute("Height")!.Value == "1000" &&
    boxLayoutActual.Descendants("TABLE").Last().Descendants("CELL").Single().Attribute("Height")!.Value == "1000",
    "Generated box auto-layout heights were not normalized.");
Check(boxLayoutActual.Descendants("TABLE").First().Element("SHAPEOBJECT")!.Element("SIZE")!.Attribute("Height")!.Value == "2000" &&
    boxLayoutActual.Descendants("TABLE").First().Descendants("CELL").Single().Attribute("Height")!.Value == "2000",
    "Generated box height normalization changed a static template table.");
Check(boxLayoutActual.Descendants("TABLE").Last().Element("SHAPEOBJECT")!.Element("SIZE")!.Attribute("Width")!.Value == "9999" &&
    boxLayoutActual.Descendants("TABLE").Last().Descendants("P").Last().Attribute("ParaShape")!.Value == "1",
    "Box layout normalization concealed a width or paragraph formatting change.");
Console.WriteLine("Box paragraph/title contract checks passed.");
static PreviewInlineContent ReadContractInlines(string json, bool allowFootnotes = true)
{
    using var document = JsonDocument.Parse(json);
    return InlineText.Read(document.RootElement, "/inlines", allowFootnotes);
}
const string contractFootnote = """
{"type":"footnote","blocks":[
  {"type":"paragraph","inlines":[{"type":"text","value":"첫째"},{"type":"space"},{"type":"strong","inlines":[{"type":"text","value":"굵게"}]},{"type":"line_break"},{"type":"emph","inlines":[{"type":"text","value":"기울임"}]}]},
  {"type":"paragraph","inlines":[{"type":"link","target":"https://example.net","title":null,"inlines":[{"type":"text","value":"링크표시"}]}]}
]}
""";
var footnoteInlines = ReadContractInlines("[{\"type\":\"text\",\"value\":\"앞\"},{\"type\":\"strong\",\"inlines\":[" + contractFootnote + "]}," + contractFootnote + ",{\"type\":\"text\",\"value\":\"뒤\"}]");
var contractNoteRuns = footnoteInlines.Lines.Single().Runs.Where(r => r.Footnote is not null).ToArray();
Check(contractNoteRuns.Length == 2 && contractNoteRuns.Select(r => r.Text).Distinct().Count() == 2, "Repeated footnote references must have independent native insertion markers.");
Check(contractNoteRuns.All(r => r.Text.StartsWith("MD2HWP_FOOTNOTE_", StringComparison.Ordinal)), "Footnote runs lack the insertion marker.");
Check(contractNoteRuns[0].Strong && !contractNoteRuns[1].Strong, "Footnote reference character formatting did not follow its enclosing span.");
Check(footnoteInlines.Lines.Single().Runs.First().Text == "앞" && footnoteInlines.Lines.Single().Runs.Last().Text == "뒤", "Adjacent body text was merged into a footnote marker or dropped.");
var noteParagraphs = contractNoteRuns[0].Footnote!.Paragraphs;
Check(noteParagraphs.Count == 2 && noteParagraphs[0].Lines.Select(l => l.Text).SequenceEqual(new[] { "첫째 굵게", "기울임" }) && noteParagraphs[1].Flatten(" / ").Text == "링크표시", "Footnote paragraphs, explicit line breaks or link labels were lost.");
Check(noteParagraphs[0].Lines[0].Runs.Last().Strong && noteParagraphs[0].Lines[1].Runs.Single().Emphasis, "Footnote content lost strong/emphasis formatting.");
var coalescedNotes = PreviewRunBuilder.Coalesce(footnoteInlines.Lines.Single().Runs);
Check(coalescedNotes.Where(r => r.Footnote is not null).Zip(contractNoteRuns).All(pair => pair.First.Text == pair.Second.Text && ReferenceEquals(pair.First.Footnote, pair.Second.Footnote)), "Run coalescing lost footnote marker associations.");
var multiLineNotes = new PreviewInlineContent(new[] { footnoteInlines.Lines.Single(), new PreviewLine("tail", new[] { new PreviewTextRun("tail", false, false) }) });
var flattenedNotes = multiLineNotes.Flatten(" / ");
Check(flattenedNotes.Text.EndsWith("뒤 / tail", StringComparison.Ordinal) && flattenedNotes.Runs.Where(r => r.Footnote is not null).Zip(contractNoteRuns).All(pair => pair.First.Text == pair.Second.Text && ReferenceEquals(pair.First.Footnote, pair.Second.Footnote)), "Inline flattening lost footnote markers or associations.");
foreach (var inline in new[] {
    "{\"type\":\"footnote\",\"blocks\":[]}",
    "{\"type\":\"footnote\",\"blocks\":[{\"type\":\"paragraph\",\"inlines\":[]}]}",
    "{\"type\":\"footnote\",\"blocks\":[{\"type\":\"heading\",\"level\":1,\"inlines\":[{\"type\":\"text\",\"value\":\"heading\"}]}]}",
    "{\"type\":\"footnote\",\"blocks\":[{\"type\":\"paragraph\",\"inlines\":[" + contractFootnote + "]}]}",
    "{\"type\":\"footnote\",\"blocks\":[{\"type\":\"paragraph\",\"inlines\":[{\"type\":\"strong\",\"inlines\":[" + contractFootnote + "]}]}]}",
    "{\"type\":\"footnote\",\"blocks\":[{\"type\":\"paragraph\",\"inlines\":[{\"type\":\"link\",\"target\":\"https://example.net\",\"title\":null,\"inlines\":[" + contractFootnote + "]}]}]}"
}) Reject(() => ReadContractInlines("[" + inline + "]"));
foreach (var wrapped in new[] { contractFootnote, "{\"type\":\"emph\",\"inlines\":[" + contractFootnote + "]}", "{\"type\":\"link\",\"target\":\"https://example.net\",\"title\":null,\"inlines\":[" + contractFootnote + "]}" })
    Reject(() => ReadContractInlines("[" + wrapped + "]", allowFootnotes: false));
var footnoteSource = new TemplateSource(BoxParagraph("source-slot"), "source-slot", "", "");
var footnoteProfile = InvestigationTemplateProfile.FromTaggedTemplate(
    typeof(IrPreviewPlan).Assembly.Location, Array.Empty<ProfileStyle>(), "fixture-reset", 100, 6, 0,
    new ProfileCaptionSelector("fixture-caption", "[", "]caption", "caption"), footnoteSource, footnoteSource,
    new TemplateListPrototype("bullet", 0, new XElement("BULLET")), new TemplateListPrototype("ordered", 0, new XElement("NUMBERING")));
var fixtureResourceRoot = new DirectoryInfo(Directory.GetCurrentDirectory());
while (!File.Exists(Path.Combine(fixtureResourceRoot.FullName, "examples", "all-features-twice", "image.png")))
    fixtureResourceRoot = fixtureResourceRoot.Parent ?? throw new Exception("Cannot find tracked PNG contract fixture.");
var fixtureIrPath = Path.Combine(fixtureResourceRoot.FullName, "examples", "all-features-twice", "contract.ir.json");
static IrPreviewPlan ReadContractBlocks(string block, string irPath, string root, InvestigationTemplateProfile profile)
{
    using var document = JsonDocument.Parse(block);
    var builder = new PlanBuilder(irPath, root, profile);
    builder.AddBlock(document.RootElement, "/blocks/0");
    return builder.Build(1);
}
foreach (var block in new[] {
    "{\"type\":\"paragraph\",\"inlines\":[" + contractFootnote + "]}",
    "{\"type\":\"heading\",\"level\":1,\"inlines\":[" + contractFootnote + "]}",
    "{\"type\":\"list\",\"kind\":\"bullet\",\"tight\":true,\"items\":[{\"blocks\":[{\"type\":\"paragraph\",\"inlines\":[" + contractFootnote + "]}]}]}"
})
{
    var plan = ReadContractBlocks(block, fixtureIrPath, fixtureResourceRoot.FullName, footnoteProfile);
    Check(plan.Operations.Single().FormattedLines!.Single().Single().Footnote?.Paragraphs.Count == 2, "A body, heading or list paragraph discarded its footnote.");
}
Reject(() => ReadContractBlocks("{\"type\":\"verbatim_block\",\"lines\":[\"raw\"],\"source\":[" + contractFootnote + "]}", fixtureIrPath, fixtureResourceRoot.FullName, footnoteProfile));
foreach (var context in new[] { "alt", "caption", "source" })
{
    const string textInline = "[{\"type\":\"text\",\"value\":\"caption\"}]";
    var noteInline = "[" + contractFootnote + "]";
    var figure = "{\"type\":\"figure\",\"image\":{\"path\":\"image.png\",\"title\":null,\"alt\":" + (context == "alt" ? noteInline : textInline) + "},\"caption\":" + (context == "caption" ? noteInline : textInline) + ",\"source\":" + (context == "source" ? noteInline : "null") + "}";
    Reject(() => ReadContractBlocks(figure, fixtureIrPath, fixtureResourceRoot.FullName, footnoteProfile));
}
Console.WriteLine("Backend footnote IR, marker and context contract checks passed.");
static XDocument FootnoteTemplate(string restart = "Continuous")
{
    var shapes = new XElement(BoxTemplate(withTitle: false).Root!.Element("HEAD")!);
    var sample = BoxParagraph(TaggedTemplateBinding.Tag("footnote"), "1", "1", "2");
    sample.SetAttributeValue("PageBreak", "false");
    sample.SetAttributeValue("ColumnBreak", "false");
    return new XDocument(new XElement("HWPML", shapes, new XElement("BODY", new XElement("SECTION",
        new XElement("P", new XElement("TEXT", new XAttribute("CharShape", "0"), new XElement("SECDEF",
            new XElement("FOOTNOTESHAPE", new XElement("AUTONUMFORMAT", new XAttribute("Type", "Digit"), new XAttribute("PrefixChar", ""), new XAttribute("SuffixChar", ")")),
                new XElement("NOTENUMBERING", new XAttribute("Type", restart), new XAttribute("NewNumber", "1")),
                new XElement("NOTEPLACEMENT", new XAttribute("Place", "BeneathText")))))),
        BoxParagraph(TaggedTemplateBinding.Tag("begin:template")), sample, BoxParagraph(TaggedTemplateBinding.Tag("end:template"))))));
}
static XElement ExistingFootnote(int number) => new("FOOTNOTE", new XElement("PARALIST", BoxParagraph("static note")));
static XDocument FootnoteRendered(XDocument template, params XElement[] paragraphs)
{
    var section = new XElement("SECTION", new XElement(AuriMinimalBoxPrototype.RootParagraphs(template)[0]), paragraphs);
    return new XDocument(new XElement("HWPML", new XElement(template.Root!.Element("HEAD")!), new XElement("BODY", section)));
}
static IrPreviewPlan NativeFootnotePlan(PreviewInlineContent content) => new("fixture", "fixture", new PreviewSummary(1, 1, 0, 0, 0),
    new[] { new PreviewOperation("text", "body", content.Lines.Select(l => l.Text).ToArray(), ParagraphStyle: "body", FormattedLines: content.Lines.Select(l => l.Runs).ToArray()) }, []);
static XElement[] NativeNoteParagraphs(XDocument document) => document.Descendants("FOOTNOTE").SelectMany(n => n.Element("PARALIST")!.Elements("P")).ToArray();
var nativeNoteTemplate = FootnoteTemplate();
var originalNativeNoteTemplate = new XDocument(nativeNoteTemplate);
var nativeNoteBinding = NativeFootnotes.Bind(nativeNoteTemplate);
var noteMarkerA = contractNoteRuns[0].Text;
var noteMarkerB = contractNoteRuns[1].Text;
var beforeNote = BoxParagraph("before " + noteMarkerA + " middle " + noteMarkerB + " after");
var nativeNoteRendered = FootnoteRendered(nativeNoteTemplate, beforeNote);
var originalNativeNoteRendered = new XDocument(nativeNoteRendered);
var attachedNotes = nativeNoteBinding.Attach(nativeNoteRendered, NativeFootnotePlan(footnoteInlines));
Check(XNode.DeepEquals(nativeNoteTemplate, originalNativeNoteTemplate) && XNode.DeepEquals(nativeNoteRendered, originalNativeNoteRendered), "Native footnote binding or attachment mutated input XML.");
Check(attachedNotes.Descendants("FOOTNOTE").Count() == 2 && !attachedNotes.ToString().Contains("MD2HWP_FOOTNOTE_", StringComparison.Ordinal), "Multiple footnotes in one CHAR were not all replaced with native controls.");
Check(TaggedTemplateBinding.DirectText(AuriMinimalBoxPrototype.RootParagraphs(attachedNotes)[1]) == "before  middle  after", "Native insertion altered text surrounding the footnote references.");
Check(attachedNotes.Descendants("AUTONUM").Select(n => (string?)n.Attribute("Number")).SequenceEqual(new[] { "1", "2" }) && attachedNotes.Descendants("AUTONUM").All(n => (string?)n.Attribute("NumberType") == "Footnote"), "Generated footnotes do not use native continuous numbering.");
var nativeNoteParagraphs = NativeNoteParagraphs(attachedNotes);
Check(nativeNoteParagraphs.Length == 4 && nativeNoteParagraphs.All(p => (string?)p.Attribute("ParaShape") == "1" && (string?)p.Attribute("Style") == "1"), "Footnotes lost the sample paragraph's formatting or paragraph boundaries.");
Check(nativeNoteParagraphs.All(p => p.Attribute("PageBreak") is null && p.Attribute("ColumnBreak") is null), "Footnote paragraphs inherited page-break flags.");
Check(nativeNoteParagraphs[0].Elements("TEXT").First().Elements().Select(e => e.Name.LocalName).SequenceEqual(new[] { "AUTONUM", "CHAR" }) && nativeNoteParagraphs[0].Elements("TEXT").First().Element("CHAR")!.Value.StartsWith(" 첫째", StringComparison.Ordinal), "The first footnote paragraph lacks native AUTONUM followed by one space.");
Check(nativeNoteParagraphs[1].Descendants("AUTONUM").Count() == 0 && TaggedTemplateBinding.DirectText(nativeNoteParagraphs[1]) == "링크표시", "A later note paragraph acquired a number or lost its link label.");
Check(nativeNoteParagraphs[0].Descendants("LINEBREAK").Count() == 1, "Explicit line break in a note became a paragraph or vanished.");
Check(attachedNotes.Descendants("AUTONUM").All(n => XNode.DeepEquals(n.Element("AUTONUMFORMAT"), nativeNoteTemplate.Descendants("FOOTNOTESHAPE").Single().Element("AUTONUMFORMAT"))), "Native note numbering format did not follow the template section.");
var boldNoteRun = nativeNoteParagraphs[0].Elements("TEXT").Single(t => t.Value == "굵게");
var italicNoteRun = nativeNoteParagraphs[0].Elements("TEXT").Single(t => t.Value == "기울임");
Check(attachedNotes.Descendants("CHARSHAPE").Single(c => (string?)c.Attribute("Id") == (string?)boldNoteRun.Attribute("CharShape")).Element("BOLD") is not null &&
    attachedNotes.Descendants("CHARSHAPE").Single(c => (string?)c.Attribute("Id") == (string?)italicNoteRun.Attribute("CharShape")).Element("ITALIC") is not null,
    "Native note rich text did not create bold/italic character shapes.");
var splitNoteReference = BoxParagraph("prefix " + noteMarkerA[..12]);
splitNoteReference.Add(new XElement("TEXT", new XAttribute("CharShape", "3"), new XElement("CHAR", noteMarkerA[12..] + " suffix")));
var singleNoteContent = new PreviewInlineContent(new[] { new PreviewLine(noteMarkerA, new[] { contractNoteRuns[0] }) });
var splitAttachedNote = nativeNoteBinding.Attach(FootnoteRendered(nativeNoteTemplate, splitNoteReference), NativeFootnotePlan(singleNoteContent));
var splitRoot = AuriMinimalBoxPrototype.RootParagraphs(splitAttachedNote)[1];
Check(splitRoot.Descendants("FOOTNOTE").Count() == 1 && TaggedTemplateBinding.DirectText(splitRoot) == "prefix  suffix" && (string?)splitRoot.Elements("TEXT").First().Attribute("CharShape") == "0", "Split-run reference changed surrounding text or its first character format.");
var noNotePlan = NativeFootnotePlan(PreviewInlineContent.Plain(new[] { "untouched" }));
Check(XNode.DeepEquals(nativeNoteRendered, nativeNoteBinding.Attach(nativeNoteRendered, noNotePlan)), "An empty native-note plan changed unrelated template XML.");
foreach (var invalidSample in new[] { "missing", "duplicate", "control", "page-break", "outside", "nested" })
{
    var invalid = FootnoteTemplate();
    var rootSamples = AuriMinimalBoxPrototype.RootParagraphs(invalid);
    var sample = rootSamples[2];
    switch (invalidSample)
    {
        case "missing": sample.Remove(); break;
        case "duplicate": sample.AddAfterSelf(new XElement(sample)); break;
        case "control": sample.Element("TEXT")!.Add(new XElement("AUTONUM")); break;
        case "page-break": sample.SetAttributeValue("PageBreak", "true"); break;
        case "outside": sample.Remove(); rootSamples[3].AddAfterSelf(sample); break;
        case "nested": sample.Remove(); rootSamples[1].Element("TEXT")!.Add(new XElement("TABLE", new XElement("ROW", new XElement("CELL", new XElement("PARALIST", sample))))); break;
    }
    Reject(() => NativeFootnotes.Bind(invalid));
}
var splitNoteSampleTemplate = FootnoteTemplate();
var splitNoteSample = AuriMinimalBoxPrototype.RootParagraphs(splitNoteSampleTemplate)[2];
splitNoteSample.Element("TEXT")!.Element("CHAR")!.Value = "{{md2hwp:foot";
splitNoteSample.Add(new XElement("TEXT", new XAttribute("CharShape", "3"), new XElement("CHAR", "note}}")));
_ = NativeFootnotes.Bind(splitNoteSampleTemplate);
foreach (var host in new[] { "HEADER", "FOOTER", "MASTERPAGE", "FOOTNOTE", "ENDNOTE" })
{
    var hostDocument = FootnoteRendered(nativeNoteTemplate, new XElement("P", new XElement("TEXT", new XAttribute("CharShape", "0"), new XElement(host, new XElement("PARALIST", BoxParagraph(noteMarkerA))))));
    Reject(() => nativeNoteBinding.Attach(hostDocument, NativeFootnotePlan(singleNoteContent)));
}
var controlSeparatedReference = BoxParagraph(noteMarkerA[..12]);
controlSeparatedReference.Element("TEXT")!.Add(new XElement("AUTONUM", new XAttribute("NumberType", "Figure"), new XAttribute("Number", "4")), new XElement("CHAR", noteMarkerA[12..]));
try { nativeNoteBinding.Attach(FootnoteRendered(nativeNoteTemplate, controlSeparatedReference), NativeFootnotePlan(singleNoteContent)); throw new Exception("A footnote marker spanning a native control was accepted."); }
catch (InvalidOperationException) { }
var noNumberingFormat = FootnoteRendered(nativeNoteTemplate, BoxParagraph(noteMarkerA));
noNumberingFormat.Descendants("AUTONUMFORMAT").Remove();
Reject(() => nativeNoteBinding.Attach(noNumberingFormat, NativeFootnotePlan(singleNoteContent)));
var pageNoteTemplate = FootnoteTemplate("OnPage");
var pageNoteBinding = NativeFootnotes.Bind(pageNoteTemplate);
var existingNote = ExistingFootnote(1);
existingNote.Descendants("TEXT").Single().AddFirst(new XElement("AUTONUM", new XAttribute("NumberType", "Footnote"), new XAttribute("Number", "1")));
var staticNumberParagraph = BoxParagraph("static");
staticNumberParagraph.Element("TEXT")!.Add(existingNote, new XElement("AUTONUM", new XAttribute("NumberType", "Figure"), new XAttribute("Number", "42")));
var pageNoteExpected = pageNoteBinding.Attach(FootnoteRendered(pageNoteTemplate, staticNumberParagraph, beforeNote), NativeFootnotePlan(footnoteInlines));
var pageNoteActual = new XDocument(pageNoteExpected);
foreach (var number in pageNoteActual.Descendants("AUTONUM")) number.SetAttributeValue("Number", "99");
pageNoteActual.Descendants("FOOTNOTE").Last().Descendants("P").First().SetAttributeValue("ParaShape", "0");
pageNoteBinding.NormalizeNumbers(pageNoteExpected, pageNoteActual);
Check(pageNoteActual.Descendants("FOOTNOTE").First().Descendants("AUTONUM").Single().Attribute("Number")!.Value == "99" && pageNoteActual.Descendants("AUTONUM").Single(n => (string?)n.Attribute("NumberType") == "Figure").Attribute("Number")!.Value == "99", "Page note normalization concealed a pre-existing note or unrelated numbering change.");
Check(pageNoteActual.Descendants("FOOTNOTE").Skip(1).Select(n => (string?)n.Descendants("AUTONUM").Single().Attribute("Number")).SequenceEqual(new[] { "2", "3" }), "Page-restart note display values were not normalized locally.");
Check(pageNoteActual.Descendants("FOOTNOTE").Last().Descendants("P").First().Attribute("ParaShape")!.Value == "0", "Page note normalization concealed paragraph formatting changes.");
var invalidPageNumber = new XDocument(pageNoteExpected);
invalidPageNumber.Descendants("FOOTNOTE").Last().Descendants("AUTONUM").Single().SetAttributeValue("Number", "0");
pageNoteBinding.NormalizeNumbers(pageNoteExpected, invalidPageNumber);
Check(invalidPageNumber.Descendants("FOOTNOTE").Last().Descendants("AUTONUM").Single().Attribute("Number")!.Value == "0", "Invalid page-based note number was silently accepted.");
var continuousExpected = nativeNoteBinding.Attach(nativeNoteRendered, NativeFootnotePlan(footnoteInlines));
var continuousActual = new XDocument(continuousExpected);
continuousActual.Descendants("AUTONUM").Last().SetAttributeValue("Number", "8");
nativeNoteBinding.NormalizeNumbers(continuousExpected, continuousActual);
Check(continuousActual.Descendants("AUTONUM").Last().Attribute("Number")!.Value == "8", "Continuous note counters were incorrectly normalized.");
var mixedNoteReference = BoxParagraph("unused");
mixedNoteReference.Element("TEXT")!.Element("CHAR")!.ReplaceNodes("before", new XElement("LINEBREAK"), "line " + noteMarkerA + " after");
var mixedNoteResult = nativeNoteBinding.Attach(FootnoteRendered(nativeNoteTemplate, mixedNoteReference), NativeFootnotePlan(singleNoteContent));
var mixedRoot = AuriMinimalBoxPrototype.RootParagraphs(mixedNoteResult)[1];
Check(mixedRoot.Descendants("FOOTNOTE").Count() == 1 && mixedRoot.Elements("TEXT").Elements("CHAR").Descendants("LINEBREAK").Count() == 1 && TaggedTemplateBinding.DirectText(mixedRoot) == "beforeline  after", "A reference alongside a native mixed-text line break was lost or changed.");
var formattedReference = BoxParagraph("before ");
formattedReference.Add(new XElement("TEXT", new XAttribute("CharShape", "0"), new XElement("CHAR", noteMarkerA)));
var mergedNoteResult = nativeNoteBinding.Attach(FootnoteRendered(nativeNoteTemplate, formattedReference), NativeFootnotePlan(singleNoteContent));
Check(AuriMinimalBoxPrototype.RootParagraphs(mergedNoteResult)[1].Elements("TEXT").Count() == 1, "Identically formatted note reference runs were not combined for native import.");
var initialNumberTemplate = FootnoteTemplate();
initialNumberTemplate.Root!.Element("HEAD")!.Add(new XElement("DOCSETTING", new XElement("BEGINNUMBER", new XAttribute("Footnote", "5"))));
initialNumberTemplate.Descendants("NOTENUMBERING").Single().SetAttributeValue("NewNumber", "7");
var initialNumberBinding = NativeFootnotes.Bind(initialNumberTemplate);
var initialNumberResult = initialNumberBinding.Attach(FootnoteRendered(initialNumberTemplate, beforeNote), NativeFootnotePlan(footnoteInlines));
Check(initialNumberResult.Descendants("AUTONUM").Select(n => (string?)n.Attribute("Number")).SequenceEqual(new[] { "5", "6" }), "Continuous notes did not inherit the document's footnote start number.");
initialNumberTemplate.Descendants("NOTENUMBERING").Single().SetAttributeValue("Type", "OnSection");
var sectionNumberResult = NativeFootnotes.Bind(initialNumberTemplate).Attach(FootnoteRendered(initialNumberTemplate, beforeNote), NativeFootnotePlan(footnoteInlines));
Check(sectionNumberResult.Descendants("AUTONUM").Select(n => (string?)n.Attribute("Number")).SequenceEqual(new[] { "7", "8" }), "Section note restart did not inherit its NewNumber.");
var restartRoot = BoxParagraph("restart");
restartRoot.Element("TEXT")!.Add(new XElement("NEWNUM", new XAttribute("NumberType", "Footnote"), new XAttribute("Number", "9")));
var restartedNotes = nativeNoteBinding.Attach(FootnoteRendered(nativeNoteTemplate, restartRoot, beforeNote), NativeFootnotePlan(footnoteInlines));
Check(restartedNotes.Descendants("AUTONUM").Select(n => (string?)n.Attribute("Number")).SequenceEqual(new[] { "9", "10" }), "A retained native footnote restart was ignored.");
var boldFirstNote = ReadContractInlines("[{\"type\":\"footnote\",\"blocks\":[{\"type\":\"paragraph\",\"inlines\":[{\"type\":\"strong\",\"inlines\":[{\"type\":\"text\",\"value\":\"bold\"}]}]}]}]");
var boldFirstResult = nativeNoteBinding.Attach(FootnoteRendered(nativeNoteTemplate, BoxParagraph(boldFirstNote.Lines.Single().Text)), NativeFootnotePlan(boldFirstNote));
var numberRun = boldFirstResult.Descendants("FOOTNOTE").Single().Descendants("AUTONUM").Single().Parent!;
Check((string?)numberRun.Attribute("CharShape") == "2" && (string?)numberRun.ElementsAfterSelf("TEXT").Single().Attribute("CharShape") != "2", "Footnote numbering acquired boldness from its first content span.");
Console.WriteLine("Native footnote structure, rich text, preserved controls and local numbering checks passed.");
FigureReferenceContractTests.Run();
TemplateCrossReferenceTests.Run();
NativeCrossReferenceTests.Run();
NativeFootnoteTests.Run();
TableContractTests.Run();
TableWidthTests.Run();
TableWidthLimitTests.Run();
CurrentParagraphStyleTests.Run();
TemplateRangeStructureTests.Run();
BoxPrototypeStructureTests.Run();
ListMarkerFormattingTests.Run();
ListContinuationTests.Run();
if (args.Length==2) {
    var expected=HancomPreviewWriter.NormalizeFigureMatrices(HwpMarkup.Parse(File.ReadAllText(args[0])));
    var actual=HancomPreviewWriter.NormalizeFigureMatrices(HwpMarkup.Parse(File.ReadAllText(args[1])));
    var before=AuriMinimalBoxPrototype.RootParagraphs(expected);
    var after=AuriMinimalBoxPrototype.RootParagraphs(actual);
    TemplateRangeStructure.RequireOriginalStyleDefinitions(expected,actual,before);
    Check(TemplateRangeStructure.Equivalent(before,after,expected,actual),"Native template changed: "+TemplateRangeStructure.DescribeDifference(before,after));
    _ = TaggedTemplateBinding.Read(TemplateMetadata.Prepare(actual,metadata).Document,args[1]);
    Check(expected.Descendants("BINDATA").Select(e=>e.Value).SequenceEqual(actual.Descendants("BINDATA").Select(e=>e.Value)),"Embedded template image data changed.");
    Console.WriteLine("Native template save/reopen structure, styles, embedded images and new heading tags preserved.");
}

ObjectSourceTests.Run();
