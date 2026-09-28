# Installing Sphatik on a Redmi Note 9 Pro

> **Status: no installable Sphatik image exists yet.** This guide describes the procedure that will be used for test builds, so testers can prepare. Follow it only with a phone you can afford to wipe.

## Before you begin

- **Supported model:** Redmi Note 9 Pro, India (codename `curtana`). Check under Settings → About phone.
- **Everything on the phone will be erased.** Back up photos, chats (WhatsApp's own backup) and files first.
- **Charge to at least 60%.**
- **You need:** a Windows or Linux computer, a USB cable, a Mi account signed in on the phone.
- **Not supported yet:** fingerprint, NFC, hardware video decoding. Banking and UPI apps will probably not work.

## Step 1: Unlock the bootloader

1. On the phone: Settings → About phone → tap MIUI version seven times to enable Developer options.
2. Developer options → OEM unlocking on; Mi Unlock status → add your account.
3. On a Windows PC, run Xiaomi's official Mi Unlock tool, boot the phone to fastboot (Volume Down + Power) and follow the tool. It usually enforces a waiting period of several days before it allows the unlock.

## Step 2: Flash the latest stock firmware

Flash the latest official MIUI fastboot ROM for curtana with Xiaomi's flashing tool, choosing **"clean all"** (not "clean all and lock"). This makes sure all firmware partitions are consistent.

## Step 3: Flash Sphatik

Download the Sphatik test build for curtana and check its SHA-256 checksum against the release page, then:

```bash
fastboot devices                              # phone listed?
fastboot flash boot sphatik-curtana-boot.img
fastboot flash userdata sphatik-curtana-userdata.img
fastboot reboot
```

The first boot shows the SKY splash and takes a few minutes while Sphatik extracts your phone's firmware and prepares storage.

## Step 4: First setup

Choose language, Wi-Fi, a PIN (this protects your encrypted data), and optionally **Settings → Android apps → Set up** to install the Android layer with microG and an app store.

## Going back to MIUI

Boot to fastboot and flash the stock fastboot ROM with Xiaomi's tool, choosing "clean all". Your phone returns to factory MIUI.

## Never do this

- Never choose "clean all and lock" while Sphatik or any non-stock image is installed.
- Never flash or erase `persist`, `modem` or other calibration partitions; you can lose IMEI, Wi-Fi or sensors permanently.

## Getting help

Report problems with the bug-report template on the issue tracker and attach the output of `sp logs --dump` if you can connect a computer.
