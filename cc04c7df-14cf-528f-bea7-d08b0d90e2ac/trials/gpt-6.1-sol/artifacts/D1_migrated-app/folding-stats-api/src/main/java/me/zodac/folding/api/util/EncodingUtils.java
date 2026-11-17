/*
 * MIT License
 *
 * Copyright (c) 2021-2022 zodac.me
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

package me.zodac.folding.api.util;

import java.nio.charset.StandardCharsets;
import java.util.Base64;
import java.util.Map;


public final class EncodingUtils {

    
    public static final String DECODED_USERNAME_KEY = "userName";

    
    public static final String DECODED_PASSWORD_KEY = "password";

    
    public static final String BASIC_AUTHENTICATION_SCHEME = "Basic ";

    private static final String DECODED_USERNAME_PASSWORD_DELIMITER = ":";

    private EncodingUtils() {

    }

    
    public static String encodeBasicAuthentication(final String userName, final String password) {
        final byte[] usernameAndPassword = (userName + DECODED_USERNAME_PASSWORD_DELIMITER + password).getBytes(StandardCharsets.ISO_8859_1);
        return BASIC_AUTHENTICATION_SCHEME + Base64
            .getEncoder()
            .encodeToString(usernameAndPassword);
    }

    
    public static boolean isInvalidBasicAuthentication(final String authorizationPayload) {
        return authorizationPayload == null || !authorizationPayload.contains(BASIC_AUTHENTICATION_SCHEME);
    }

    
    public static Map<String, String> decodeBasicAuthentication(final String authorizationPayload) {
        if (authorizationPayload == null) {
            throw new IllegalArgumentException("Cannot decode null");
        }

        if (!authorizationPayload.startsWith(BASIC_AUTHENTICATION_SCHEME)) {
            throw new IllegalArgumentException(String.format("Cannot decode input that does not start with: '%s'", BASIC_AUTHENTICATION_SCHEME));
        }

        final String encodedUserNameAndPassword = authorizationPayload.split(BASIC_AUTHENTICATION_SCHEME)[1];
        return decodeAuthentication(encodedUserNameAndPassword);
    }

    
    public static Map<String, String> decodeAuthentication(final String encodedUserNameAndPassword) {
        if (encodedUserNameAndPassword == null) {
            throw new IllegalArgumentException("Cannot decode null");
        }

        final String decodedUserNameAndPassword = new String(Base64.getDecoder().decode(encodedUserNameAndPassword), StandardCharsets.ISO_8859_1);

        if (!decodedUserNameAndPassword.contains(DECODED_USERNAME_PASSWORD_DELIMITER)) {
            throw new IllegalArgumentException(String.format("Decoded input does not contain: '%s'", DECODED_USERNAME_PASSWORD_DELIMITER));
        }

        final String[] userNameAndPasswordTokens = decodedUserNameAndPassword.split(DECODED_USERNAME_PASSWORD_DELIMITER, 2);

        return Map.of(
            DECODED_USERNAME_KEY, userNameAndPasswordTokens[0],
            DECODED_PASSWORD_KEY, userNameAndPasswordTokens[1]
        );
    }
}
