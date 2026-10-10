using System.Text.Json;
using System.Xml.Linq;
using Md2Hwp.HancomIrPreview;

internal static class FigureDimensionsTests
{
    internal static void Run()
    {
        void Check(bool value, string message) { if (!value) throw new Exception(message); }
        var root = new DirectoryInfo(Directory.GetCurrentDirectory());
        while (!File.Exists(Path.Combine(root.FullName,"examples","jpeg","image.jpg")))
            root = root.Parent ?? throw new Exception("Missing tracked JPEG fixtures.");
        var resources = Path.Combine(root.FullName,"examples","jpeg");
        foreach (var name in new[] { "image.jpg", "image.jpeg" })
            Check(FigureDimensions.Read(Path.Combine(resources,name)) == (480,320), "JPEG dimensions/aspect ratio lost.");
        Check(FigureDimensions.Read(Path.Combine(root.FullName,"examples","all-features","image.png")) is { Width: > 0, Height: > 0 }, "PNG support regressed.");
        var source = new TemplateSource(new XElement("P"), "slot", "", "");
        var profile = InvestigationTemplateProfile.FromTaggedTemplate(typeof(IrPreviewPlan).Assembly.Location, [], "reset", 142, 6, 0,
            new ProfileCaptionSelector("caption", "[", "]", "slot"),source,source,
            new TemplateListPrototype("bullet",0,new XElement("BULLET")),new TemplateListPrototype("ordered",0,new XElement("NUMBERING")));
        foreach (var name in new[] { "image.jpg", "image.jpeg" })
        {
            using var input = JsonDocument.Parse($$"""{"type":"figure","image":{"path":"{{name}}","title":null,"alt":[{"type":"text","value":"JPEG"}]},"caption":[{"type":"text","value":"caption"}],"source":null}""");
            var builder = new PlanBuilder(Path.Combine(resources,"fixture.ir.json"),resources,profile);
            builder.AddBlock(input.RootElement,"/blocks/0");
            var operation = builder.Build(1).Operations.Single();
            Check(operation.ImagePath == Path.Combine(resources,name) && operation.ImageWidthMillimeters == 142 &&
                Math.Abs(operation.ImageHeightMillimeters!.Value - 142.0*320/480) < 0.00001, "JPEG figure plan lost path or aspect ratio.");
        }
        var temporary = Path.Combine(Path.GetTempPath(), "md2hwp-jpeg-"+Guid.NewGuid().ToString("N")+".JPEG");
        try
        {
            byte[] Frame(byte marker) => [0xff,0xd8,0xff,marker,0,17,8,1,64,1,224,3,1,0x11,0,2,0x11,0,3,0x11,0];
            void Read(byte[] bytes) { File.WriteAllBytes(temporary,bytes); Check(FigureDimensions.Read(temporary) == (480,320), "Uppercase extension, JPEG frame or metadata skip failed."); }
            Read(Frame(0xc0)); Read(Frame(0xc1)); Read(Frame(0xc2));
            Read([0xff,0xd8,0xff,0xe1,0,6,0xff,0xc0,0,0,..Frame(0xc2)[2..]]);
            Read([0xff,0xd8,0xff,0x01,0xff,..Frame(0xc0)[2..]]);
            void Reject(byte[] bytes) {
                File.WriteAllBytes(temporary,bytes);
                try { FigureDimensions.Read(temporary); } catch (InvalidDataException) { return; }
                throw new Exception("Malformed JPEG header was accepted.");
            }
            Reject([]); Reject([0xff]); Reject([0xff,0xd8]);
            Reject([0xff,0xd8,0xff,0xe0,0,1]); // Length includes its own two bytes.
            Reject([0xff,0xd8,0xff,0xe0,0,20,0]);
            Reject([0xff,0xd8,0xff,0xda,0,2]); Reject([0xff,0xd8,0xff,0xd9]);
            Reject([0xff,0xd8,0xff,0,0,0]); Reject([0xff,0xd8,0xff,0xc0,0,8,8,1,64,1,224,3]);
            var zero = Frame(0xc0); zero[7]=0;zero[8]=0;Reject(zero);
            var precision = Frame(0xc0); precision[6]=12;Reject(precision);
            Reject(Frame(0xc3));
            Reject(File.ReadAllBytes(Path.Combine(root.FullName,"examples","all-features","image.png")));
        }
        finally { File.Delete(temporary); }
        Console.WriteLine("PNG/JPG/JPEG signatures, sequential/progressive headers, resource plans, aspect ratios and malformed input contracts passed.");
    }
}
