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

package me.zodac.folding.api.tc;

import java.util.Objects;
import lombok.AccessLevel;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.EqualsAndHashCode;
import lombok.Getter;
import lombok.ToString;
import me.zodac.folding.api.ResponsePojo;
import me.zodac.folding.rest.api.tc.request.UserRequest;



@AllArgsConstructor(access = AccessLevel.PRIVATE)
@Builder
@Getter
@EqualsAndHashCode
@ToString(doNotUseGetters = true)
public class User implements ResponsePojo {

    
    public static final int EMPTY_USER_ID = 0;

    private static final int PASSKEY_LENGTH_NOT_TO_HIDE = 8;
    private static final String PASSKEY_MASK = "************************"; // 24 characters

    private final int id;
    private final String foldingUserName;
    private final String displayName;
    private final String passkey;
    private final Category category;
    private final String profileLink;
    private final String liveStatsLink;
    private final Hardware hardware;
    private final Team team;
    private final boolean userIsCaptain;

    
    public static User create(final int userId,
                              final String foldingUserName,
                              final String displayName,
                              final String passkey,
                              final Category category,
                              final String profileLink,
                              final String liveStatsLink,
                              final Hardware hardware,
                              final Team team,
                              final boolean isCaptain) {
        final String profileLinkOrNull = isEmpty(profileLink) ? null : profileLink;
        final String liveStatsLinkOrNull = isEmpty(liveStatsLink) ? null : liveStatsLink;
        return new User(userId, foldingUserName, displayName, passkey, category, profileLinkOrNull, liveStatsLinkOrNull, hardware, team, isCaptain);
    }

    
    public static User createWithoutId(final String foldingUserName,
                                       final String displayName,
                                       final String passkey,
                                       final Category category,
                                       final String profileLink,
                                       final String liveStatsLink,
                                       final Hardware hardware,
                                       final Team team,
                                       final boolean isCaptain) {
        return create(EMPTY_USER_ID, foldingUserName, displayName, passkey, category, profileLink, liveStatsLink, hardware, team, isCaptain);
    }

    
    public static User createWithoutId(final UserRequest userRequest, final Hardware hardware, final Team team) {
        return createWithoutId(
            userRequest.getFoldingUserName(),
            userRequest.getDisplayName(),
            userRequest.getPasskey(),
            Category.get(userRequest.getCategory()),
            userRequest.getProfileLink(),
            userRequest.getLiveStatsLink(),
            hardware,
            team,
            userRequest.isUserIsCaptain()
        );
    }

    
    public static User updateWithId(final int userId, final User user) {
        return create(userId, user.foldingUserName, user.displayName, user.passkey, user.category, user.profileLink, user.liveStatsLink,
            user.hardware, user.team, user.userIsCaptain);
    }

    
    public static User updateHardware(final User user, final Hardware hardware) {
        return create(user.id, user.foldingUserName, user.displayName, user.passkey, user.category, user.profileLink, user.liveStatsLink, hardware,
            user.team, user.userIsCaptain);
    }

    
    public static User updateTeam(final User user, final Team team) {
        return create(user.id, user.foldingUserName, user.displayName, user.passkey, user.category, user.profileLink, user.liveStatsLink,
            user.hardware, team, user.userIsCaptain);
    }

    
    public static User removeCaptaincyFromUser(final User user) {
        return create(user.id, user.foldingUserName, user.displayName, user.passkey, user.category, user.profileLink, user.liveStatsLink,
            user.hardware, user.team, false);
    }

    
    public static User hidePasskey(final User user) {
        return create(user.id, user.foldingUserName, user.displayName, hidePasskey(user.passkey), user.category, user.profileLink, user.liveStatsLink,
            user.hardware, user.team, user.userIsCaptain);
    }

    private static String hidePasskey(final String passkey) {
        final int endIndex = Math.min(PASSKEY_LENGTH_NOT_TO_HIDE, passkey.length()); // In case passkey has fewer than 8 characters, though unlikely
        return passkey.substring(0, endIndex).concat(PASSKEY_MASK);
    }

    private static boolean isEmpty(final String input) {
        return input == null || input.isBlank();
    }

    
    public boolean isEqualRequest(final UserRequest userRequest) {
        return hardware.getId() == userRequest.getHardwareId()
            && team.getId() == userRequest.getTeamId()
            && userIsCaptain == userRequest.isUserIsCaptain()
            && Objects.equals(foldingUserName, userRequest.getFoldingUserName())
            && Objects.equals(displayName, userRequest.getDisplayName())
            && Objects.equals(passkey, userRequest.getPasskey())
            && Objects.equals(category.toString(), userRequest.getCategory())
            && Objects.equals(profileLink, userRequest.getProfileLink())
            && Objects.equals(liveStatsLink, userRequest.getLiveStatsLink());
    }
}
