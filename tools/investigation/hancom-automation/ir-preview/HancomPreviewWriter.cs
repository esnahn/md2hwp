using System.Diagnostics;
using System.Net;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text.Json;
using System.Xml.Linq;
using Microsoft.Win32;

namespace Md2Hwp.HancomIrPreview;

internal sealed record ProbeResult(
    string Template,
    string ModulePath,
    bool RegisterModule,
    bool Open,
    bool TemplateUnchanged);

internal sealed record RenderResult(
    string Output,
    int TextOperations,
    int FigureOperations,
    int PicturesAdded,
    bool TextMarkerVerified,
    bool TemplateUnchanged);

internal static class HancomPreviewWriter
{
    private const string ProgId = "HWPFrame.HwpObject";
    private const string ModuleName = "FilePathCheckerModuleExample";
    private const string OpenOptions = "lock:false;forceopen:true;suspendpassword:true;versionwarning:false";

    public static ProbeResult Probe(string templatePath, string repositoryRoot, bool visible)
    {
        var template = ValidateTemplate(templatePath);
        EnsureInteractiveContext();
        EnsureNoExistingHwpProcess();
        var module = SecurityModuleRegistration.ReadAndValidate(repositoryRoot);
        var hashBefore = HashFile(template);

        WithHwp(module, hwp =>
        {
            Open(hwp, template, visible);
            return true;
        });

        var unchanged = string.Equals(hashBefore, HashFile(template), StringComparison.Ordinal);
        if (!unchanged)
        {
            throw new InvalidOperationException("The open-only probe changed its source template.");
        }
        return new ProbeResult(template, module.ModulePath, true, true, true);
    }

    public static RenderResult Render(
        IrPreviewPlan plan,
        string templatePath,
        string outputPath,
        string repositoryRoot,
        bool visible)
    {
        var template = ValidateTemplate(templatePath);
        var output = Path.GetFullPath(outputPath);
        if (!string.Equals(Path.GetExtension(output), ".hwp", StringComparison.OrdinalIgnoreCase))
        {
            throw new ArgumentException("The investigation preview currently preserves HWP only.");
        }
        if (string.Equals(template, output, StringComparison.OrdinalIgnoreCase))
        {
            throw new ArgumentException("Output must not overwrite the source template.");
        }
        if (File.Exists(output))
        {
            throw new IOException($"Output already exists: {output}");
        }
        var outputDirectory = Path.GetDirectoryName(output)!;
        if (!Directory.Exists(outputDirectory))
        {
            throw new DirectoryNotFoundException($"Missing output directory: {outputDirectory}");
        }

        EnsureInteractiveContext();
        EnsureNoExistingHwpProcess();
        var module = SecurityModuleRegistration.ReadAndValidate(repositoryRoot);
        var templateHashBefore = HashFile(template);
        var temporaryOutput = Path.Combine(
            outputDirectory,
            $".md2hwp-ir-preview-{Guid.NewGuid():N}.hwp");
        File.Copy(template, temporaryOutput, overwrite: false);

        try
        {
            var marker = $"MD2HWP_IR_PREVIEW_{Guid.NewGuid():N}";
            var picturesAdded = WithHwp(module, hwp =>
            {
                Open(hwp, temporaryOutput, visible);
                var picturesBefore = CountPictures(hwp);
                Run(hwp, "MoveDocEnd");
                Run(hwp, "BreakPara");
                InsertText(hwp, marker);
                Run(hwp, "BreakPara");

                foreach (var operation in plan.Operations)
                {
                    RenderOperation(hwp, operation);
                }

                Run(hwp, "FileSave");
                CloseDocument(hwp);
                Open(hwp, temporaryOutput, visible);

                var extracted = WebUtility.HtmlDecode((string)hwp.GetTextFile("TEXT", ""));
                if (!extracted.Contains(marker, StringComparison.Ordinal))
                {
                    throw new InvalidOperationException("Saved preview did not contain its verification marker.");
                }
                var picturesAfter = CountPictures(hwp);
                var added = picturesAfter - picturesBefore;
                if (added != plan.Summary.FigureOperations)
                {
                    throw new InvalidOperationException(
                        $"Expected {plan.Summary.FigureOperations} inserted pictures, observed {added}.");
                }
                return added;
            });

            var templateUnchanged = string.Equals(
                templateHashBefore,
                HashFile(template),
                StringComparison.Ordinal);
            if (!templateUnchanged)
            {
                throw new InvalidOperationException("The source template changed during preview rendering.");
            }

            File.Move(temporaryOutput, output);
            return new RenderResult(
                output,
                plan.Summary.TextOperations,
                plan.Summary.FigureOperations,
                picturesAdded,
                true,
                true);
        }
        finally
        {
            if (File.Exists(temporaryOutput))
            {
                File.Delete(temporaryOutput);
            }
        }
    }

    private static T WithHwp<T>(SecurityModuleRegistration module, Func<dynamic, T> operation)
    {
        _ = module;
        object? comObject = null;
        dynamic? hwp = null;
        Exception? primaryFailure = null;
        Exception? cleanupFailure = null;
        T? result = default;

        try
        {
            var type = Type.GetTypeFromProgID(ProgId, throwOnError: true)!;
            comObject = Activator.CreateInstance(type)
                ?? throw new InvalidOperationException($"Could not create {ProgId}.");
            hwp = comObject;

            // This must remain the first COM call after object creation.
            if (!(bool)hwp.RegisterModule("FilePathCheckDLL", ModuleName))
            {
                throw new InvalidOperationException("Hancom rejected the registered file-access security module.");
            }

            result = operation(hwp);
        }
        catch (Exception error)
        {
            primaryFailure = error;
        }
        finally
        {
            if (hwp is not null)
            {
                try
                {
                    _ = hwp.Clear(1);
                }
                catch (Exception error)
                {
                    cleanupFailure = error;
                }
                try
                {
                    _ = hwp.Quit();
                }
                catch (Exception error)
                {
                    cleanupFailure = cleanupFailure is null
                        ? error
                        : new AggregateException(cleanupFailure, error);
                }
            }
            if (comObject is not null && Marshal.IsComObject(comObject))
            {
                _ = Marshal.FinalReleaseComObject(comObject);
            }
            GC.Collect();
            GC.WaitForPendingFinalizers();
        }

        Exception? exitFailure = null;
        try
        {
            WaitForHwpExit();
        }
        catch (Exception error)
        {
            exitFailure = error;
        }

        if (primaryFailure is not null)
        {
            throw new InvalidOperationException(
                "Hancom preview operation failed.",
                Combine(primaryFailure, cleanupFailure, exitFailure));
        }
        if (cleanupFailure is not null || exitFailure is not null)
        {
            throw new InvalidOperationException(
                "Hancom preview cleanup failed.",
                Combine(cleanupFailure, exitFailure));
        }
        return result!;
    }

    private static Exception Combine(params Exception?[] errors)
    {
        var present = errors.Where(error => error is not null).Cast<Exception>().ToArray();
        return present.Length == 1 ? present[0] : new AggregateException(present);
    }

    private static void Open(dynamic hwp, string path, bool visible)
    {
        if (!(bool)hwp.Open(path, "HWP", OpenOptions))
        {
            throw new InvalidOperationException($"Hancom could not open: {path}");
        }
        if (visible)
        {
            hwp.XHwpWindows.Item(0).Visible = true;
        }
    }

    private static void CloseDocument(dynamic hwp)
    {
        _ = hwp.Clear(1);
    }

    private static void RenderOperation(dynamic hwp, PreviewOperation operation)
    {
        if (operation.Kind == "text")
        {
            for (var index = 0; index < operation.Lines.Count; index++)
            {
                InsertText(hwp, $"[{operation.Label}{(index == 0 ? string.Empty : $".line-{index + 1}")}] ");
                if (operation.Lines[index].Length > 0)
                {
                    InsertText(hwp, operation.Lines[index]);
                }
                Run(hwp, "BreakPara");
            }
            return;
        }
        if (operation.Kind != "figure" || operation.ImagePath is null ||
            operation.ImageWidthMillimeters is null || operation.ImageHeightMillimeters is null)
        {
            throw new InvalidOperationException($"Invalid preview operation: {operation.Kind}");
        }

        InsertText(hwp, "[figure] " + operation.Lines[0]);
        Run(hwp, "BreakPara");
        object? insertionResult = hwp.InsertPicture(
            operation.ImagePath,
            true,
            1,
            false,
            false,
            0,
            operation.ImageWidthMillimeters.Value,
            operation.ImageHeightMillimeters.Value);
        var inserted = insertionResult switch
        {
            bool value => value,
            null => false,
            _ => Marshal.IsComObject(insertionResult),
        };
        if (!inserted)
        {
            var resultType = insertionResult?.GetType().FullName ?? "null";
            throw new InvalidOperationException(
                $"Hancom returned an unexpected InsertPicture result ({resultType}): {operation.ImagePath}");
        }
        Run(hwp, "MoveParaEnd");
        Run(hwp, "BreakPara");
        InsertText(hwp, "[figure.caption] " + operation.Lines[1]);
        Run(hwp, "BreakPara");
        InsertText(hwp, "[figure.source] " + operation.Lines[2]);
        Run(hwp, "BreakPara");
    }

    private static void InsertText(dynamic hwp, string text)
    {
        _ = hwp.HAction.GetDefault("InsertText", hwp.HParameterSet.HInsertText.HSet);
        hwp.HParameterSet.HInsertText.Text = text;
        if (!(bool)hwp.HAction.Execute("InsertText", hwp.HParameterSet.HInsertText.HSet))
        {
            throw new InvalidOperationException("Hancom failed to insert preview text.");
        }
    }

    private static void Run(dynamic hwp, string action)
    {
        if (!(bool)hwp.HAction.Run(action))
        {
            throw new InvalidOperationException($"Hancom action failed: {action}");
        }
    }

    private static int CountPictures(dynamic hwp)
    {
        var xml = (string)hwp.GetTextFile("HWPML2X", "");
        return XDocument.Parse(xml)
            .Descendants()
            .Count(element => element.Name.LocalName == "PICTURE");
    }

    private static string ValidateTemplate(string path)
    {
        var fullPath = Path.GetFullPath(path);
        if (!File.Exists(fullPath))
        {
            throw new FileNotFoundException("Missing HWP template.", fullPath);
        }
        if (!string.Equals(Path.GetExtension(fullPath), ".hwp", StringComparison.OrdinalIgnoreCase))
        {
            throw new ArgumentException("The investigation preview currently supports HWP input only.");
        }
        return fullPath;
    }

    private static void EnsureInteractiveContext()
    {
        if (!OperatingSystem.IsWindows() || !Environment.Is64BitProcess || !Environment.UserInteractive ||
            Process.GetCurrentProcess().SessionId == 0 ||
            Thread.CurrentThread.GetApartmentState() is not ApartmentState.STA)
        {
            throw new InvalidOperationException(
                "Hancom preview requires an interactive Windows x64 STA process outside session 0.");
        }
    }

    private static void EnsureNoExistingHwpProcess()
    {
        var processes = Process.GetProcessesByName("Hwp");
        if (processes.Length > 0)
        {
            throw new InvalidOperationException(
                "Close existing HWP processes before the isolated preview: " +
                string.Join(", ", processes.Select(process => process.Id)));
        }
    }

    private static void WaitForHwpExit()
    {
        var deadline = DateTime.UtcNow.AddSeconds(15);
        while (DateTime.UtcNow < deadline && Process.GetProcessesByName("Hwp").Length > 0)
        {
            Thread.Sleep(200);
        }
        var remaining = Process.GetProcessesByName("Hwp");
        if (remaining.Length > 0)
        {
            throw new InvalidOperationException(
                "HWP remained after COM cleanup; no process was force-stopped: " +
                string.Join(", ", remaining.Select(process => process.Id)));
        }
    }

    private static string HashFile(string path) =>
        Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));

}

internal sealed record SecurityModuleRegistration(string ModulePath, string Sha256)
{
    public static SecurityModuleRegistration ReadAndValidate(string repositoryRoot)
    {
        var lockPath = Path.Combine(repositoryRoot, "dependencies", "lock.json");
        using var lockDocument = JsonDocument.Parse(File.ReadAllBytes(lockPath));
        var pins = lockDocument.RootElement.GetProperty("dependencies")
            .EnumerateArray()
            .Where(dependency => dependency.GetProperty("name").GetString() == "hancom-automation")
            .SelectMany(dependency => dependency.GetProperty("pins").EnumerateArray())
            .Where(pin => pin.GetProperty("name").GetString() == "file-path-checker-module-example")
            .ToArray();
        if (pins.Length != 1)
        {
            throw new InvalidOperationException("Expected one locked Hancom security module.");
        }
        var expectedHash = pins[0].GetProperty("sha256").GetString()!;

        using var registryKey = Registry.CurrentUser.OpenSubKey(
            @"Software\HNC\HwpAutomation\Modules",
            writable: false);
        if (registryKey is null || !registryKey.GetValueNames().Contains(ModuleName, StringComparer.Ordinal))
        {
            throw new InvalidOperationException("The Hancom security-module registration is missing.");
        }
        if (registryKey.GetValueKind(ModuleName) is not RegistryValueKind.String)
        {
            throw new InvalidOperationException("The Hancom security-module registration must be REG_SZ.");
        }
        var registeredPath = registryKey.GetValue(
            ModuleName,
            null,
            RegistryValueOptions.DoNotExpandEnvironmentNames) as string;
        if (string.IsNullOrWhiteSpace(registeredPath) || !Path.IsPathRooted(registeredPath))
        {
            throw new InvalidOperationException("The Hancom security-module path must be absolute.");
        }
        var fullPath = Path.GetFullPath(registeredPath);
        if (!File.Exists(fullPath))
        {
            throw new FileNotFoundException("The registered Hancom security module is missing.", fullPath);
        }
        var actualHash = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(fullPath)));
        if (!string.Equals(actualHash, expectedHash, StringComparison.Ordinal))
        {
            throw new InvalidOperationException("The registered Hancom security module hash does not match dependencies/lock.json.");
        }
        return new SecurityModuleRegistration(fullPath, actualHash);
    }

    private const string ModuleName = "FilePathCheckerModuleExample";
}
