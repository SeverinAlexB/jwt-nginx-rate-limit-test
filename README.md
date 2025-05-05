# Nginx JWT Rate Limiting

This repository demonstrates how to implement user-based rate limiting in Nginx by extracting the subject claim (`sub`) from JWT tokens.

## How It Works

### Extracting JWT Subject in Nginx

1. Use `ngx_http_js_module` to process JWTs:
   ```javascript
   // Extract the 'sub' claim from a JWT token in the authorization cookie
   function extractSubFromJwt(r) {
       const cookies = r.headersIn.cookie || '';
       const authCookieMatch = cookies.match(/authorization=([^;]+)/);
       
       if (!authCookieMatch || !authCookieMatch[1]) {
           return null;
       }
       
       const jwt = authCookieMatch[1];
       const parts = jwt.split('.');
       const payload = JSON.parse(decodeBase64Url(parts[1]));
       
       return payload.sub || null;
   }

   // Return user ID or IP address as fallback
   function getUserId(r) {
       return extractSubFromJwt(r) || r.remoteAddress;
   }
   ```

2. Import in Nginx config:
   ```nginx
   js_import /etc/nginx/conf.d/extract_sub.js;
   js_set $user_id extract_sub.getUserId;
   ```

### Implementing Rate Limiting

1. Define rate limiting zones:
   ```nginx
   # Limit requests per minute per user
   limit_req_zone $user_id zone=user_limit:10m rate=2r/m;
   
   # Limit concurrent connections per user
   limit_conn_zone $user_id zone=conn_limit_per_user:10m;
   ```

2. Apply rate limits to endpoints:
   ```nginx
   # Limit requests per minute
   location /fetch {
       limit_req zone=user_limit nodelay;
       proxy_pass "http://localhost:3000/fetch";
   }
   
   # Limit concurrent connections and bandwidth
   location /download {
       limit_conn conn_limit_per_user 1;
       limit_rate 100k;
       proxy_pass "http://localhost:3000/download";
   }
   
   # Limit concurrent uploads per user
   location /upload {
       client_max_body_size 10M;
       limit_conn conn_limit_per_user 1;
       proxy_pass "http://localhost:3000/upload";
   }
   ```

## Supported Rate Limiting Strategies

1. **Request Rate Limiting**: Limit requests per time period (e.g., 2 requests per minute)
2. **Connection Limiting**: Limit concurrent connections per user
3. **Bandwidth Limiting**: Restrict download speeds (e.g., 100KB/s)
4. **Upload Limiting**: Limit to 1 concurrent connection per user

## Testing

The included test suite verifies:
- JWT authentication and subject extraction
- Request rate limiting behavior
- Download speed restrictions
- Upload functionality

Run tests with:
```bash
cargo test
```

## Requirements

- Nginx with JavaScript module (`ngx_http_js_module`) support
- JWT authentication system that sets an `authorization` cookie

