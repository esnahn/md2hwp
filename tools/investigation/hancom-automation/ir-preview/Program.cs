using System.Text.Json;

namespace Md2Hwp.HancomIrPreview;

internal static class Program
{
    [STAThread]
    public static int Main(string[] args)
    {
        try
        {
            var options = CommandLine.Parse(args);
            var repositoryRoot = RepositoryLocator.FindFrom(Directory.GetCurrentDirectory());

            switch (options.Mode)
            {
                case OperationMode.Plan:
                    {
                        var profile = InvestigationTemplateProfile.Load(
                            options.ProfilePath!,
                            repositoryRoot);
                        var plan = IrPreviewPlan.Load(options.IrPath!, repositoryRoot, profile);
                        Console.WriteLine(JsonSerializer.Serialize(plan, JsonOutput.Options));
                        break;
                    }
                case OperationMode.Probe:
                    {
                        var result = HancomPreviewWriter.Probe(
                            options.TemplatePath!,
                            repositoryRoot,
                            options.Visible);
                        Console.WriteLine(JsonSerializer.Serialize(result, JsonOutput.Options));
                        break;
                    }
                case OperationMode.Render:
                    {
                        var profile = InvestigationTemplateProfile.Load(
                            options.ProfilePath!,
                            repositoryRoot);
                        var plan = IrPreviewPlan.Load(options.IrPath!, repositoryRoot, profile);
                        var result = HancomPreviewWriter.Render(
                            plan,
                            profile,
                            options.TemplatePath!,
                            options.OutputPath!,
                            repositoryRoot,
                            options.Visible);
                        Console.WriteLine(JsonSerializer.Serialize(result, JsonOutput.Options));
                        break;
                    }
                case OperationMode.ExportImages:
                    {
                        var result = HancomPreviewWriter.ExportImages(
                            options.DocumentPath!,
                            options.OutputPath!,
                            repositoryRoot,
                            options.Visible);
                        Console.WriteLine(JsonSerializer.Serialize(result, JsonOutput.Options));
                        break;
                    }
                default:
                    throw new InvalidOperationException($"Unknown mode: {options.Mode}");
            }

            return 0;
        }
        catch (Exception error)
        {
            WriteError(error);
            return 1;
        }
    }

    private static void WriteError(Exception error, int depth = 0)
    {
        var prefix = depth == 0 ? "hancom-ir-preview: " : $"{new string(' ', depth * 2)}caused by ";
        Console.Error.WriteLine($"{prefix}{error.GetType().Name}: {error.Message}");
        if (error.StackTrace is not null)
        {
            Console.Error.WriteLine(error.StackTrace);
        }

        if (error is AggregateException aggregate)
        {
            foreach (var inner in aggregate.InnerExceptions)
            {
                WriteError(inner, depth + 1);
            }
        }
        else if (error.InnerException is not null)
        {
            WriteError(error.InnerException, depth + 1);
        }
    }
}

internal static class JsonOutput
{
    public static readonly JsonSerializerOptions Options = new()
    {
        WriteIndented = true,
    };
}

internal enum OperationMode
{
    Plan,
    Probe,
    Render,
    ExportImages,
}

internal sealed record CommandLine(
    OperationMode Mode,
    string? IrPath,
    string? ProfilePath,
    string? TemplatePath,
    string? DocumentPath,
    string? OutputPath,
    bool Visible)
{
    public static CommandLine Parse(string[] args)
    {
        if (args.Length == 0)
        {
            throw new ArgumentException(Usage);
        }

        var mode = args[0] switch
        {
            "plan" => OperationMode.Plan,
            "probe" => OperationMode.Probe,
            "render" => OperationMode.Render,
            "export-images" => OperationMode.ExportImages,
            _ => throw new ArgumentException(Usage),
        };
        string? ir = null;
        string? profile = null;
        string? template = null;
        string? document = null;
        string? output = null;
        var visible = false;

        for (var index = 1; index < args.Length; index++)
        {
            switch (args[index])
            {
                case "--ir":
                    ir = ReadValue(args, ref index, "--ir");
                    break;
                case "--profile":
                    profile = ReadValue(args, ref index, "--profile");
                    break;
                case "--template":
                    template = ReadValue(args, ref index, "--template");
                    break;
                case "--document":
                    document = ReadValue(args, ref index, "--document");
                    break;
                case "--output":
                    output = ReadValue(args, ref index, "--output");
                    break;
                case "--visible":
                    visible = true;
                    break;
                default:
                    throw new ArgumentException($"Unknown argument {args[index]}.\n{Usage}");
            }
        }

        var valid = mode switch
        {
            OperationMode.Plan => ir is not null && profile is not null && template is null && document is null && output is null && !visible,
            OperationMode.Probe => ir is null && profile is null && template is not null && document is null && output is null,
            OperationMode.Render => ir is not null && profile is not null && template is not null && document is null && output is not null,
            OperationMode.ExportImages => ir is null && profile is null && template is null && document is not null && output is not null,
            _ => false,
        };
        if (!valid)
        {
            throw new ArgumentException(Usage);
        }

        return new CommandLine(
            mode,
            ResolveOptionalPath(ir),
            ResolveOptionalPath(profile),
            ResolveOptionalPath(template),
            ResolveOptionalPath(document),
            ResolveOptionalPath(output),
            visible);
    }

    private static string ReadValue(string[] args, ref int index, string option)
    {
        index++;
        if (index >= args.Length || args[index].StartsWith("--", StringComparison.Ordinal))
        {
            throw new ArgumentException($"Missing value for {option}.\n{Usage}");
        }
        return args[index];
    }

    private static string? ResolveOptionalPath(string? path) =>
        path is null ? null : Path.GetFullPath(path);

    private const string Usage = """
        usage:
          hancom-ir-preview plan --ir <validated.ir.json> --profile <template-profile.json>
          hancom-ir-preview probe --template <input.hwp> [--visible]
          hancom-ir-preview render --ir <validated.ir.json> --profile <template-profile.json> --template <input.hwp> --output <new.hwp> [--visible]
          hancom-ir-preview export-images --document <input.hwp> --output <new-directory> [--visible]
        """;
}

internal static class RepositoryLocator
{
    public static string FindFrom(string start)
    {
        for (var directory = new DirectoryInfo(Path.GetFullPath(start));
             directory is not null;
             directory = directory.Parent)
        {
            if (File.Exists(Path.Combine(directory.FullName, "global.json")) &&
                File.Exists(Path.Combine(directory.FullName, "dependencies", "lock.json")))
            {
                return directory.FullName;
            }
        }
        throw new InvalidOperationException("Could not locate the md2hwp repository root.");
    }
}
