# Oath OAuth
Serverless OAuth Sessions demo app

```bash
# build and deploy
sam build
sam deploy --guided

#logs
sam logs --stack-name <stackname> --name <FnName>
```

### Githuyb OAuth App
Authorized callback Url 
```
https://<your-deployment>.execute-api.<your region>.amazonaws.com/Prod/login/github/callback
```
