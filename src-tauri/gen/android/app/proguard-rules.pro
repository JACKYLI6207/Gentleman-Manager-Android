# Add project specific ProGuard rules here.
# You can control the set of applied configuration files using the
# proguardFiles setting in build.gradle.

# Keep custom Android plugin classes used by Rust register_android_plugin.
-keep class com.gentleman.manager.android.FolderPickerPlugin { *; }
-keep class com.gentleman.manager.android.ReadArgs { *; }
-keep class com.gentleman.manager.android.TreeArgs { *; }
-keep class com.gentleman.manager.android.CopyToTreeArgs { *; }
-keep class com.gentleman.manager.android.AppendLineArgs { *; }
