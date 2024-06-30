## What is it?
This a program to gather public steam profiles that have {1,1500} reviews and add their friends, reviews , games

## How does it do it?
1. This program acts as a webcrawler through the friends list of the users
2. It checks(when visiting the profile) whether it can see the widgets of reviews,freinds,games along the side of the profile
3. goes through the list of visibility checks to visit those profiles reviews(one page at a time :( 10 at a time), reach out to the steam api for games list
4. add each one (even those with no visibility to cut down on website visit counts) to a sqlite dataset
5. repeat through one of those people with freinds visable or reach back into the dataset for someone who does have them available but not used friendslist
   
## Have you ran it?
I collected over 600,000 public profiles and peformed a few tests inside of that data directory of this project

## Why Collect this information?
1. I thought it was cool and fun project
2. I also built another project out of this called Steam Rec AI that tries to guess which games you would play and recommend most.

# Where can I get my hands on this dataset
I uploaded the data onto kaggle as a thanks for using a previous steam account list. You can find it [here](https://www.kaggle.com/datasets/bossadapt/public-steam-users-reviews-games-and-friends)
