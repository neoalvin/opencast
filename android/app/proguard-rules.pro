# Add project specific ProGuard rules here.

# Ktor
-keep class io.ktor.** { *; }
-dontwarn io.ktor.**

# OkHttp
-dontwarn okhttp3.**
-keep class okhttp3.** { *; }

# Netty
-dontwarn io.netty.**
-keep class io.netty.** { *; }
