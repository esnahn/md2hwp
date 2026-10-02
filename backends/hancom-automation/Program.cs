using System.Reflection;
using System.Text.Json;

namespace Md2Hwp.HancomIrPreview;

internal static class Program
{
    [STAThread]
    public static int Main(string[] args)
    {
        try
        {
            if (args is ["--version"])
            {
                var version = typeof(Program).Assembly
                    .GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion
                    ?? throw new InvalidOperationException("Missing application version metadata.");
                Console.WriteLine($"md2hwp-backend {version.Split('+')[0]}");
                return 0;
            }
            if (args.Length > 0 && args[0] == "init-template")
            {
                var output = args.Length switch
                {
                    1 => "template.hwp",
                    2 when !args[1].StartsWith("--", StringComparison.Ordinal) => args[1],
                    3 when args[1] == "--output" && !args[2].StartsWith("--", StringComparison.Ordinal) => args[2],
                    _ => throw new ArgumentException("usage: md2hwp-backend init-template [[--output] <template.hwp>]"),
                };
                Console.WriteLine(JsonSerializer.Serialize(HancomPreviewWriter.CreateDefaultTemplate(output), JsonOutput.Options));
                return 0;
            }
            if (args is ["runtime-info"])
            {
                Console.WriteLine(JsonSerializer.Serialize(new {
                    Framework = "Microsoft.NETCore.App", Version = Environment.Version.ToString(),
                    Architecture = System.Runtime.InteropServices.RuntimeInformation.ProcessArchitecture.ToString()
                }, JsonOutput.Options));
                return 0;
            }
            var options = CommandLine.Parse(args);
            var resourceRoot = CommandLine.IsPositional(args)
                ? Path.GetDirectoryName(options.IrPath)! : Directory.GetCurrentDirectory();
            Console.WriteLine(JsonSerializer.Serialize(HancomPreviewWriter.RenderTaggedTemplate(
                options.IrPath, options.TemplatePath, options.OutputPath, resourceRoot, options.Visible, options.Verbose), JsonOutput.Options));
            return 0;
        }
        catch (Exception error)
        {
            Console.Error.WriteLine($"md2hwp-backend: {error}");
            return 1;
        }
    }
}

internal static class JsonOutput
{
    public static readonly JsonSerializerOptions Options = new() { WriteIndented = true };
}

internal sealed record CommandLine(string IrPath, string TemplatePath, string OutputPath, bool Visible, bool Verbose)
{
    internal static bool IsPositional(string[] args) => args.Length > 0 &&
        !args[0].StartsWith("--", StringComparison.Ordinal) && args[0].EndsWith(".json", StringComparison.OrdinalIgnoreCase);

    public static CommandLine Parse(string[] args)
    {
        if (IsPositional(args))
        {
            var input = Path.GetFullPath(args[0]);
            var name = Path.GetFileName(input);
            var stem = name.EndsWith(".ir.json", StringComparison.OrdinalIgnoreCase)
                ? name[..^8] : Path.GetFileNameWithoutExtension(name);
            string? result = null;
            var forwarded = new List<string> { "ir2hwp", "--ir", input };
            var positionalOptions = new HashSet<string>(StringComparer.Ordinal);
            for (var index = 1; index < args.Length; index++)
            {
                var option = args[index];
                if (option is "--output" or "--template")
                {
                    if (!positionalOptions.Add(option)) throw new ArgumentException($"Duplicate argument: {option}");
                    if (++index >= args.Length || args[index].StartsWith("--", StringComparison.Ordinal))
                        throw new ArgumentException($"Missing value for {option}.\n{Usage}");
                    if (option == "--output")
                    {
                        if (result is not null) throw new ArgumentException("Duplicate output argument.");
                        result = args[index];
                    }
                    else forwarded.AddRange([option, args[index]]);
                }
                else if (option is "--visible" or "--verbose") forwarded.Add(option);
                else
                {
                    if (option.StartsWith("-", StringComparison.Ordinal) || result is not null) throw new ArgumentException(Usage);
                    result = option;
                }
            }
            forwarded.AddRange(["--output", result ?? Path.Combine(Path.GetDirectoryName(input)!, stem + ".output.hwp")]);
            return Parse(forwarded.ToArray());
        }
        if (args.Length == 0 || args[0] != "ir2hwp") throw new ArgumentException(Usage);
        string? ir = null, template = null, output = null;
        var visible = false;
        var verbose = false;
        var seen = new HashSet<string>(StringComparer.Ordinal);
        for (var index = 1; index < args.Length; index++)
        {
            var option = args[index];
            if (!seen.Add(option)) throw new ArgumentException($"Duplicate argument: {option}");
            if (option == "--visible") { visible = true; continue; }
            if (option == "--verbose") { verbose = true; continue; }
            if (option is not ("--ir" or "--template" or "--output")) throw new ArgumentException(Usage);
            if (++index >= args.Length || args[index].StartsWith("--", StringComparison.Ordinal))
                throw new ArgumentException($"Missing value for {option}.\n{Usage}");
            switch (option)
            {
                case "--ir": ir = args[index]; break;
                case "--template": template = args[index]; break;
                case "--output": output = args[index]; break;
            }
        }
        if (ir is null || output is null) throw new ArgumentException(Usage);
        return new(Path.GetFullPath(ir), Path.GetFullPath(template ?? Path.Combine(AppContext.BaseDirectory, "template.hwp")),
            Path.GetFullPath(output), visible, verbose);
    }
    private const string Usage = """
        usage:
          md2hwp-backend init-template [[--output] <template.hwp>]
          md2hwp-backend <source.ir.json> [[--output] <source.output.hwp>] [--template <template.hwp>] [--visible] [--verbose]
          md2hwp-backend ir2hwp --ir <source.ir.json> --output <source.output.hwp> [--template <template.hwp>] [--visible] [--verbose]
          md2hwp-backend --version
        Default template: template.hwp beside the executable.
        """;
}
