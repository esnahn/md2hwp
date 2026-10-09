using System.Reflection;
using System.Text.Json;
using Md2Hwp.HancomIrPreview;

internal static class RenderTimingTests
{
    public static void Run()
    {
        var oldTiming = Environment.GetEnvironmentVariable("MD2HWP_TIMING");
        var oldProfile = Environment.GetEnvironmentVariable("MD2HWP_PROFILE");
        var oldError = Console.Error;
        try
        {
            using var output = new StringWriter();
            Console.SetError(output);
            Environment.SetEnvironmentVariable("MD2HWP_TIMING", null);
            if (RenderTiming.Start() is not null) throw new Exception("Timing must be opt-in.");
            Environment.SetEnvironmentVariable("MD2HWP_TIMING", "1");
            using (var timing = RenderTiming.Start()!) { timing.Complete(); }
            CheckTiming(output.ToString(), true);
            output.GetStringBuilder().Clear();
            using (RenderTiming.Start()) { }
            CheckTiming(output.ToString(), false);
#if DEBUG
            output.GetStringBuilder().Clear();
            Environment.SetEnvironmentVariable("MD2HWP_PROFILE", "1");
            using (var profile = RenderProfile.Start()!)
            {
                using (RenderProfile.Measure("test.phase")) { }
                profile.Complete();
            }
            var line = output.ToString().Trim();
            using var json = JsonDocument.Parse(line["md2hwp-profile: ".Length..]);
            if (!json.RootElement.GetProperty("Completed").GetBoolean() ||
                json.RootElement.GetProperty("Metrics")[0].GetProperty("Name").GetString() != "test.phase")
                throw new Exception("Debug detailed profiling was lost.");
#else
            if (typeof(HancomPreviewWriter).Assembly.GetType("Md2Hwp.HancomIrPreview.RenderProfile") is not null)
                throw new Exception("Detailed profiling must not be compiled into Release.");
            foreach (var name in new[] { "ReadDocument", "ReadBlock" })
                if (typeof(HwpMarkup).GetMethod(name, BindingFlags.Static | BindingFlags.NonPublic)!.GetParameters().Length != 1)
                    throw new Exception("Release XML readers must not retain caller profiling metadata.");
#endif
        }
        finally
        {
            Console.SetError(oldError);
            Environment.SetEnvironmentVariable("MD2HWP_TIMING", oldTiming);
            Environment.SetEnvironmentVariable("MD2HWP_PROFILE", oldProfile);
        }
        Console.WriteLine("Opt-in total timing, failure status and Debug-only detailed profiling contracts passed.");
    }

    private static void CheckTiming(string output, bool completed)
    {
        using var json = JsonDocument.Parse(output.Trim()["md2hwp-time: ".Length..]);
        var root = json.RootElement;
        if (root.EnumerateObject().Count() != 2 || root.GetProperty("Completed").GetBoolean() != completed ||
            root.GetProperty("TotalMilliseconds").GetDouble() < 0)
            throw new Exception("Total timing must report completion and elapsed time only.");
    }
}
