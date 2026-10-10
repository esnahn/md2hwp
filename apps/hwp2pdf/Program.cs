using System.Reflection;
using Md2Hwp.HancomIrPreview;

namespace Md2Hwp.Pdf;

internal static class Program
{
    [STAThread]
    public static int Main(string[] args)
    {
        try
        {
            if (args is ["--help"] or ["-h"])
            {
                Console.WriteLine(PdfExportOptions.Usage);
                return 0;
            }
            if (args is ["--version"])
            {
                var version = typeof(Program).Assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()
                    ?.InformationalVersion ?? throw new InvalidOperationException("Missing application version metadata.");
                Console.WriteLine($"hwp2pdf {version.Split('+')[0]}");
                return 0;
            }
            var result = HancomPreviewWriter.ExportPdf(PdfExportOptions.Parse(args));
            Console.WriteLine(result.Output);
            return 0;
        }
        catch (Exception error)
        {
            Console.Error.WriteLine($"hwp2pdf: {error.GetBaseException().Message}");
            return 1;
        }
    }
}
