# הוראות העלאת Firebase Service Account

## שלב 1: הורדת Service Account Key מ-Firebase

1. היכנס ל-Firebase Console: https://console.firebase.google.com/project/messageapp-40141
2. לחץ על ⚙️ (Settings) → **Project Settings**
3. עבור לכרטיסייה **Service Accounts**
4. לחץ על **Generate new private key**
5. אשר ולחץ **Generate key**
6. שמור את הקובץ JSON (לדוגמה: `messageapp-40141-firebase-adminsdk.json`)

## שלב 2: העלאה ל-AWS Secrets Manager

הרץ את הפקודה הבאה ב-PowerShell (החלף את הנתיב לקובץ שהורדת):

```powershell
# קרא את תוכן ה-JSON
$firebaseKey = Get-Content "C:\path\to\messageapp-40141-firebase-adminsdk.json" -Raw

# העלה ל-Secrets Manager
aws secretsmanager put-secret-value `
  --secret-id "arn:aws:secretsmanager:us-east-1:221671810590:secret:eyedar-prod-firebase-key-21cx4i" `
  --secret-string $firebaseKey `
  --region us-east-1
```

## שלב 3: אימות

```powershell
# בדוק שהסוד הועלה בהצלחה
aws secretsmanager describe-secret `
  --secret-id "arn:aws:secretsmanager:us-east-1:221671810590:secret:eyedar-prod-firebase-key-21cx4i" `
  --region us-east-1
```

אם הפקודה מחזירה פרטים על הסוד (כולל LastUpdatedDate חדש), ההעלאה הצליחה!

## מידע נוסף

- **Project ID**: messageapp-40141
- **Secret ARN**: arn:aws:secretsmanager:us-east-1:221671810590:secret:eyedar-prod-firebase-key-21cx4i
- **Region**: us-east-1

---

**לאחר השלמת שלב זה, המערכת תוכל לשלוח התראות FCM לנהגים! 📱**
