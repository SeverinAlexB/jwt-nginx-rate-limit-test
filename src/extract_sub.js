/**
 * Extract the 'sub' (user ID) from a JWT token stored in the authorization cookie
 * For use with nginx and ngx_http_js_module
 */
function extractSubFromJwt(r) {
    // Get the authorization cookie
    const cookies = r.headersIn.cookie || '';
    const authCookieMatch = cookies.match(/authorization=([^;]+)/);
    
    if (!authCookieMatch || !authCookieMatch[1]) {
        r.error("No authorization cookie found");
        return null;
    }
    
    const jwt = authCookieMatch[1];
    
    try {
        // JWT format: header.payload.signature
        // Split the JWT and get the payload (second part)
        const parts = jwt.split('.');
        if (parts.length !== 3) {
            r.error("Invalid JWT format");
            return null;
        }
        
        // Base64 decode the payload
        const payload = JSON.parse(decodeBase64Url(parts[1]));
        
        // Extract the sub claim
        if (!payload.sub) {
            r.error("No 'sub' claim found in JWT");
            return null;
        }
        
        return payload.sub;
    } catch (error) {
        r.error("Error extracting sub from JWT: " + error.message);
        return null;
    }
}

/**
 * Function to be used with js_set directive
 * Returns user ID for rate limiting or a default value if not found
 */
function getUserId(r) {
    const userId = extractSubFromJwt(r);
    // Return the user ID if found, or IP address as fallback for rate limiting
    return userId || r.remoteAddress;
}

/**
 * Decode base64url encoded string
 */
function decodeBase64Url(str) {
    // Convert base64url to base64
    let base64 = str.replace(/-/g, '+').replace(/_/g, '/');
    
    // Add padding if needed
    while (base64.length % 4) {
        base64 += '=';
    }
    
    // Decode base64 to binary string
    const binaryStr = atob(base64);
    
    // Convert binary string to UTF-8 string
    let result = '';
    for (let i = 0; i < binaryStr.length; i++) {
        result += String.fromCharCode(binaryStr.charCodeAt(i));
    }
    
    return result;
}

// Export the functions for use in nginx configuration
export default {extractSubFromJwt, getUserId};
