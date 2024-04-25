## What is it
This a program to gather public steam profiles that have {1,1500} reviews and add their freinds, reviews , games

## How does it do it
1. This program acts as a webcrawler through the freinds list of the users
2. It checks(when visiting the profile) whether it can see the widgets of reviews,freinds,games along the side of the profile
3. goes through the list of visibility checks to visit those profiles reviews(one page at a time :( 10 at a time), reach out to the steam api for games list
4. add each one (even those with no visibility to cut down on website visit counts) to a sqlite dataset
5. repeat through one of those people with freinds visable or reach back into the dataset for someone who does have them available but not used freindslist

## Whats next
currently I have over 500,000 profiles with games grabbed and over 2,000,000 profiles(alot of them nonpublic/dont have enough reviews) visited in total. I will create a recommendation system and likely publish my dataset so others can use it.
