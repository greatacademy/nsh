package com.longbridge.nsh;

import android.app.Activity;
import android.content.Context;
import android.content.pm.PackageManager;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;

import java.lang.ref.WeakReference;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Runtime permissions that an app's own Rust crate asks for ({@code nsh_platform_android::permissions}), as the
 * microphone for audio it captures itself. The app's manifest must declare each one; one it does not is denied without
 * a prompt. The prompt shows over the last activity that made an {@link NshView}.
 */
public final class Permissions {
    private static final String TAG_RESULTS = "com.longbridge.nsh.Permissions";

    /** The activity of the last view made. UI thread. */
    private static WeakReference<Activity> current = new WeakReference<>(null);
    /** The application's context, once a view was made. */
    private static volatile Context application;

    private Permissions() {}

    /** Remembers the activity of a view being made. UI thread. */
    static void attach(Context context) {
        application = context.getApplicationContext();
        Activity activity = SystemUi.activity(context);
        if (activity != null) current = new WeakReference<>(activity);
    }

    /** The application's context, or null before a view was made. Any thread. */
    public static Context context() {
        return application;
    }

    /** Whether the app has {@code permission}. Any thread. */
    public static boolean granted(String permission) {
        Context context = application;
        return context != null && context.checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED;
    }

    /** Asks for {@code permission} if the app does not have it, answering {@code token}. Any thread. */
    static void request(String permission, long token) {
        new Handler(Looper.getMainLooper()).post(() -> {
            if (granted(permission)) {
                answer(token, true);
                return;
            }
            Activity activity = current.get();
            if (activity == null || activity.isFinishing() || activity.isDestroyed()) {
                answer(token, false);
                return;
            }
            results(activity).ask(permission, token);
        });
    }

    /** The system's answer for {@code token}. */
    static native void answer(long token, boolean granted);

    @SuppressWarnings("deprecation")
    private static Results results(Activity activity) {
        android.app.FragmentManager fragments = activity.getFragmentManager();
        Results results = (Results) fragments.findFragmentByTag(TAG_RESULTS);
        if (results == null) {
            results = new Results();
            fragments.beginTransaction().add(results, TAG_RESULTS).commitNow();
        }
        return results;
    }

    /** Gets the system's answers to the permission prompts, across configuration changes. */
    @SuppressWarnings("deprecation")
    public static final class Results extends android.app.Fragment {
        private static final int CODE = 0x4f01;
        /** The tokens waiting, by permission. */
        private final Map<String, List<Long>> waiting = new HashMap<>();

        @Override
        public void onCreate(Bundle state) {
            super.onCreate(state);
            setRetainInstance(true);
        }

        void ask(String permission, long token) {
            List<Long> tokens = waiting.get(permission);
            if (tokens != null) {
                tokens.add(token);
                return;
            }
            tokens = new ArrayList<>();
            tokens.add(token);
            waiting.put(permission, tokens);
            requestPermissions(new String[] {permission}, CODE);
        }

        @Override
        public void onRequestPermissionsResult(int code, String[] permissions, int[] results) {
            if (code != CODE) {
                super.onRequestPermissionsResult(code, permissions, results);
                return;
            }
            // Empty: the prompt was cancelled, which answers no to everything asked.
            if (permissions.length == 0) {
                Map<String, List<Long>> all = new HashMap<>(waiting);
                waiting.clear();
                for (List<Long> tokens : all.values()) for (long t : tokens) answer(t, false);
                return;
            }
            for (int i = 0; i < permissions.length; i++) {
                List<Long> tokens = waiting.remove(permissions[i]);
                if (tokens == null) continue;
                boolean granted = i < results.length && results[i] == PackageManager.PERMISSION_GRANTED;
                for (long t : tokens) answer(t, granted);
            }
        }
    }
}
